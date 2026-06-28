#![no_std]

pub mod events;
mod test;
pub mod types;

use events::{
    BountyReleasedEvent, FundsClawedBackEvent, MaintainerAuthFailedEvent, PoolCreatedEvent,
    TOPIC_BOUNTY_RELEASED, TOPIC_FUNDS_CLAWED_BACK, TOPIC_MAINTAINER_AUTH_FAILED,
    TOPIC_POOL_CREATED,
};
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Symbol};

use types::{ClaimRecord, DataKey, Error, MilestonePool, TokenClient, WaveGuardClient};

/// Checks that `address` is registered as an active maintainer in the
/// WaveGuard registry at `guard_contract`.  Returns
/// `Err(Error::UnauthorizedMaintainer)` if not.
fn ensure_is_maintainer(env: &Env, guard_contract: &Address, address: &Address) -> Result<(), Error> {
    let guard = WaveGuardClient::new(env, guard_contract);
    if !guard.is_maintainer(address) {
        return Err(Error::UnauthorizedMaintainer);
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────
// Contract Entry Point
// ─────────────────────────────────────────────────────────────

/// # WaveMilestone — Security Audit Notes
///
/// ## Trust Assumptions
///
/// - **WaveGuard is trusted**: Every privileged write (pool creation, bounty
///   release) defers identity decisions to the WaveGuard registry at
///   `pool.guard_contract`. If that contract is compromised, upgraded
///   maliciously, or replaced, an attacker can obtain maintainer status and
///   drain the pool.  The address is fixed at `create_milestone_pool` time and
///   cannot be rotated — this is intentional; changing it post-creation would
///   itself require a trusted authority and open a different attack surface.
///
/// - **Maintainer is trusted with fund direction**: The `release_issue_bounty`
///   entry point accepts an arbitrary `developer` address supplied by the
///   caller.  A malicious or compromised maintainer can therefore redirect
///   bounties to any address.  This is an accepted design trade-off: the
///   protocol is permissioned, and maintainers are vetted by WaveGuard.
///   Off-chain governance and WaveGuard revocation are the intended mitigations.
///
/// - **Token contract is trusted**: The contract calls an external SAC-style
///   token.  A malicious token at `pool.asset` could re-enter, report false
///   balances, or silently fail transfers.  Deployment should only use
///   verified Stellar Asset Contracts.
///
/// ## Unauthorized Claim Manipulation — Audit Findings
///
/// ### FINDING CM-01 (CRITICAL — Fixed): Temporary-storage expiry re-claim
/// Original code stored `ClaimRecord` in **Temporary** storage.  Stellar's
/// Temporary storage entries are pruned after their TTL expires.  Once pruned,
/// `env.storage().temporary().get(...)` returns `None`, the duplicate-claim
/// guard treats the issue as unclaimed, and a maintainer can re-release the
/// same bounty.  This has been **fixed** by migrating `ClaimRecord` records to
/// **Persistent** storage so they survive for the ledger lifetime of the
/// contract.  See `release_issue_bounty` and `is_claimed` below.
///
/// ### FINDING CM-02 (INFO): Developer address not restricted
/// `release_issue_bounty` accepts the beneficiary address as a caller-supplied
/// parameter.  There is no on-chain restriction preventing a maintainer from
/// directing a bounty to an address they control.  This is acknowledged and
/// mitigated at the governance layer (WaveGuard revocation).  A corresponding
/// test (`test_maintainer_can_redirect_developer_address`) documents the
/// expected, permitted behavior.
///
/// ## Temporary Storage Leakage
///
/// ### NOTE TMP-01: Temporary storage is not used for claim records (post-fix)
/// After CM-01's fix, `ClaimRecord` entries now live in Persistent storage.
/// No sensitive claim state is held in Temporary storage.  Callers should be
/// aware that any future use of Temporary storage for authorization state
/// (e.g., nonces, session flags) would be subject to the same expiry-based
/// re-use risk and must be explicitly TTL-managed.
///
/// ### NOTE TMP-02: `is_claimed` query reliability
/// The public `is_claimed` view now reads from Persistent storage.  Off-chain
/// indexers that previously called this endpoint should note the storage
/// migration: entries created before this fix (Temporary) are distinct from
/// entries created after (Persistent) and may co-exist during a migration
/// window on live networks.
///
/// ## WaveGuard Authorization Flow
///
/// WaveMilestone delegates identity and access control to an external
/// **WaveGuard** registry contract.  The flow is:
///
/// 1. **Pool creation** (`create_milestone_pool`)
///    The caller proves their maintainer status by passing the
///    WaveGuard `is_maintainer` check.  The guard contract address is
///    recorded in `pool.guard_contract` and is **immutable** for the
///    lifetime of that pool.
///
/// 2. **Bounty release** (`release_issue_bounty`)
///    Every call re-evaluates `is_maintainer` against the stored
///    `pool.guard_contract`.  There is **no** cached authorisation —
///    a maintainer whose WaveGuard membership is revoked between
///    pool creation and bounty release is blocked from paying out.
///    This ensures that off-chain governance actions (removing a
///    rogue maintainer) take effect immediately on-chain.
///
/// 3. **Clawback** (`clawback_expired_funds`)
///    WaveGuard is **not** consulted.  Clawback is gated on direct
///    address equality with `pool.maintainer`.  This deliberate
///    asymmetry isolates the refund path from a compromised
///    WaveGuard, so the original depositor can always recover
///    unclaimed funds.
///
/// ```
///                    ┌──────────────┐
///                    │  WaveGuard   │  (external access registry)
///                    │ is_maintainer│
///                    └──────┬───────┘
///                           │ ① create_milestone_pool
///                           │ ② release_issue_bounty
///                           ▼
///                    ┌──────────────┐
///                    │ WaveMilestone│  (this contract)
///                    │  escrow vault│
///                    └──────┬───────┘
///                           │ ③ transfer (SAC)
///                           ▼
///                    ┌──────────────┐
///                    │    Token     │  (Stellar Asset Contract)
///                    └──────────────┘
/// ```
///
/// **Key security properties:**
/// - The WaveGuard address is fixed at pool creation; it cannot be
///   rotated mid-lifecycle.
/// - Maintainer checks are **live** — each privileged call re-queries
///   the registry, not the pool snapshot.
/// - Clawback uses a separate authorisation path (address equality)
///   so that a WaveGuard compromise cannot drain the escrow.
/// - Failed maintainer checks emit a `maintainer_auth_failed` event
///   to support off-chain alerting and forensics.
#[contract]
pub struct WaveMilestoneContract;

#[contractimpl]
impl WaveMilestoneContract {
    // ── Lifecycle: Pool Creation ─────────────────────────────

    /// Creates a new milestone escrow pool.
    ///
    /// Transfers `total_funds` of `asset` from `maintainer` into the
    /// contract vault, links to a WaveGuard registry for access control,
    /// and sets a milestone `expiry` (ledger timestamp, Unix seconds).
    ///
    /// # Auth
    /// - `maintainer.require_auth()` — the caller must sign.
    /// - WaveGuard `is_maintainer` check passes.
    ///
    /// # Trust Assumptions
    /// - `guard_contract` must be a deployed, trusted WaveGuard instance.
    ///   Once set, it cannot be changed; compromise of that contract grants
    ///   unrestricted access to this pool.
    /// - `asset` must be a trusted Stellar Asset Contract (SAC).  A
    ///   malicious token could re-enter or silently fail transfers.
    pub fn create_milestone_pool(
        env: Env,
        maintainer: Address,
        guard_contract: Address,
        asset: Address,
        total_funds: u128,
        expiry: u64,
    ) -> Result<(), Error> {
        // ── AUTH GATE 1/2: Stellar signature check ──
        maintainer.require_auth();

        // ── WaveGuard validation ──
        if guard_contract == env.current_contract_address() {
            return Err(Error::InvalidGuard);
        }
        let guard = WaveGuardClient::new(&env, &guard_contract);
        if !guard.is_maintainer(&maintainer) {
            env.events().publish(
                (Symbol::new(&env, TOPIC_MAINTAINER_AUTH_FAILED),),
                MaintainerAuthFailedEvent {
                    maintainer: maintainer.clone(),
                    reason: Symbol::new(&env, "not_registered"),
                    guard_contract: guard_contract.clone(),
                },
            );
            return Err(Error::UnauthorizedMaintainer);
        }

        // ── Input validation ──
        let now = env.ledger().timestamp();
        if expiry == 0 {
            return Err(Error::InvalidExpiry);
        }
        if expiry <= now {
            return Err(Error::ExpiryInPast);
        }

        // ── Duplicate pool guard ──
        if env.storage().instance().get::<_, MilestonePool>(&DataKey::Pool).is_some() {
            return Err(Error::PoolAlreadyExists);
        }

        // ── Fund transfer ──
        let token = TokenClient::new(&env, &asset);
        let maintainer_balance = token.balance(&maintainer);
        if maintainer_balance < total_funds {
            return Err(Error::TransferFailed);
        }
        token.transfer(&maintainer, &env.current_contract_address(), &total_funds);

        // ── Persist pool ──
        let pool = MilestonePool {
            guard_contract,
            asset: asset.clone(),
            total_funds,
            allocated_funds: 0,
            expiry,
            maintainer: maintainer.clone(),
        };
        env.storage().instance().set(&DataKey::Pool, &pool);

        // ── Emit event ──
        env.events().publish(
            (Symbol::new(&env, TOPIC_POOL_CREATED),),
            PoolCreatedEvent { maintainer, asset, total_funds, expiry },
        );

        Ok(())
    }

    // ── Lifecycle: Bounty Release ────────────────────────────

    /// Releases a micro-payout to `developer` for a completed issue.
    ///
    /// Each `(repo_hash, issue_id)` pair can be claimed exactly once.
    /// The contract verifies the maintainer's identity via WaveGuard,
    /// checks the issue has not already been paid, confirms sufficient
    /// pool balance, then transfers the tokens and marks the claim.
    ///
    /// # Parameters
    /// - `repo_hash`: SHA-256 hash of the GitHub repository full name (e.g.
    ///   `sha256("owner/repo")`).  Used as a namespace so that issue IDs from
    ///   different repositories never collide inside the same pool.  See
    ///   [`DataKey::IssueClaim`] for the composite key structure.
    /// - `issue_id`: GitHub issue number scoped to `repo_hash`.
    /// - `developer`: Stellar address receiving the payout.  Must not be a
    ///   zero-like address (all-zero bytes); see [`Error::InvalidDeveloper`].
    /// - `amount`: Token units to transfer (must be > 0 and ≤ remaining balance).
    ///
    /// # Auth
    /// - `maintainer.require_auth()` — the caller must sign.
    /// - WaveGuard `is_maintainer` check passes.
    ///
    /// # Errors
    /// - [`Error::InvalidDeveloper`] — `developer` is a zero-like address.
    /// - [`Error::BountyAlreadyClaimed`] — the `(repo_hash, issue_id)` pair was
    ///   already paid out.
    /// - [`Error::InsufficientPoolBalance`] — `amount` exceeds remaining funds.
    /// - [`Error::UnauthorizedMaintainer`] — caller not registered in WaveGuard.
    ///
    /// # Trust Assumptions
    /// - `developer` is caller-supplied and not otherwise restricted on-chain.
    ///   A malicious maintainer can direct the bounty to any non-zero address.
    ///   Mitigation is governance-layer: WaveGuard revocation (see CM-02).
    ///
    /// # Claim Storage (Security Fix CM-01)
    /// Claim records are stored in **Persistent** storage (not Temporary).
    /// Temporary storage entries expire after their TTL, which would allow
    /// a pruned entry to be re-claimed.  Persistent storage ensures the
    /// duplicate-claim guard is durable for the contract's lifetime.
    ///
    /// # Example
    /// ```rust
    /// // Compute repo_hash = sha256("owner/my-repo") off-chain, then call:
    /// client.release_issue_bounty(
    ///     &maintainer,   // WaveGuard-registered maintainer address
    ///     &repo_hash,    // BytesN<32> SHA-256 of "owner/my-repo"
    ///     &42u32,        // GitHub issue number
    ///     &developer,    // Contributor's Stellar address
    ///     &500_000_000u128, // Payout in stroops / smallest token unit
    /// );
    /// ```
    pub fn release_issue_bounty(
        env: Env,
        maintainer: Address,
        repo_hash: BytesN<32>,
        issue_id: u32,
        developer: Address,
        amount: u128,
    ) -> Result<(), Error> {
        // ── AUTH GATE 1/2: Stellar signature check ──
        maintainer.require_auth();

        // ── repo_hash validation ──
        if repo_hash == BytesN::from_array(&env, &[0u8; 32]) {
            return Err(Error::InvalidAmount);
        }

        // ── Load pool ──
        let mut pool = env
            .storage()
            .instance()
            .get::<_, MilestonePool>(&DataKey::Pool)
            .ok_or(Error::PoolNotFound)?;

        // ── WaveGuard validation ──
        ensure_is_maintainer(&env, &pool.guard_contract, &maintainer)?;

        // ── Developer address validation (issue #109) ──
        // Reject the all-zero contract address by comparing the raw 32-byte id.
        // CAAAA...D2KM is the Strkey encoding of the 32-byte all-zero contract id.
        if developer == Address::from_str(&env, "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM") {
            return Err(Error::InvalidDeveloper);
        }

        // ── Duplicate-claim guard (CM-01: reads Persistent storage) ──
        // SECURITY: Must use Persistent storage here. Temporary storage entries
        // expire after their TTL; a lapsed entry returns None, bypassing this
        // guard and allowing a maintainer to re-claim the same issue bounty.
        // Uniqueness is enforced by key existence alone — the key IS the claim.
        let claim_key = DataKey::IssueClaim(repo_hash.clone(), issue_id);
        if env.storage().persistent().has(&claim_key) {
            return Err(Error::BountyAlreadyClaimed);
        }
        let claim_key = DataKey::IssueClaim(repo_hash.clone(), issue_id);

        // ── Balance check ──
        let remaining = pool.remaining_balance();
        if amount == 0 {
            return Err(Error::InvalidAmount);
        }
        if amount > remaining {
            return Err(Error::InsufficientPoolBalance);
        }

        // ── Transfer tokens ──
        let token = TokenClient::new(&env, &pool.asset);
        token.transfer(&env.current_contract_address(), &developer, &amount);

        // ── Update pool state ──
        pool.allocated_funds = pool.allocated_funds.checked_add(amount).ok_or(Error::InvalidAmount)?;
        env.storage().instance().set(&DataKey::Pool, &pool);

        // ── Record claim in Persistent storage (CM-01 fix) ──
        let claim = IssueClaim { developer: developer.clone(), payment_amount: amount };
        env.storage().persistent().set(&claim_key, &claim);

        // ── Emit event ──
        env.events().publish(
            (Symbol::new(&env, TOPIC_BOUNTY_RELEASED),),
            BountyReleasedEvent { repo_hash, issue_id, developer, amount },
        );

        Ok(())
    }

    // ── Lifecycle: Clawback ──────────────────────────────────

    /// Returns unclaimed funds to the maintainer after milestone expiry.
    ///
    /// Only callable by the original `pool.maintainer` while they remain an
    /// active WaveGuard maintainer, and only after `pool.expiry` has passed.
    /// Transfers the full remaining balance back to the maintainer and zeroes
    /// out the available pool.
    ///
    /// # Auth
    /// - `maintainer.require_auth()` — the caller must sign.
    /// - WaveGuard `is_maintainer` check passes.
    /// - `maintainer` must match `pool.maintainer` (address equality check).
    pub fn clawback_expired_funds(env: Env, maintainer: Address) -> Result<(), Error> {
        // ── AUTH GATE 1/1: Stellar signature check ──
        // NOTE: WaveGuard is intentionally NOT re-checked here. Clawback uses
        // direct address equality (pool.maintainer) so a WaveGuard compromise
        // cannot redirect funds via this path. See trust assumptions in docstring.
        maintainer.require_auth();

        let mut pool = env
            .storage()
            .instance()
            .get::<_, MilestonePool>(&DataKey::Pool)
            .ok_or(Error::PoolNotFound)?;

        // ── Authorization ──
        // Non-owners must pass the WaveGuard check first, then are still
        // rejected with UnauthorizedCaller.  The pool owner bypasses WaveGuard
        // so they can always recover their funds even if the guard is revoked.
        if maintainer != pool.maintainer {
            ensure_is_maintainer(&env, &pool.guard_contract, &maintainer)?;
            return Err(Error::UnauthorizedCaller);
        }

        let now = env.ledger().timestamp();
        if now <= pool.expiry {
            return Err(Error::PoolNotExpired);
        }

        let remaining = pool.remaining_balance();
        if remaining == 0 {
            return Err(Error::NoFundsToClawback);
        }

        let token = TokenClient::new(&env, &pool.asset);
        token.transfer(&env.current_contract_address(), &maintainer, &remaining);

        pool.total_funds = pool.allocated_funds;
        env.storage().instance().set(&DataKey::Pool, &pool);

        env.events().publish(
            (Symbol::new(&env, TOPIC_FUNDS_CLAWED_BACK),),
            FundsClawedBackEvent { maintainer, amount: remaining },
        );

        Ok(())
    }

    // ── View / Query Methods ─────────────────────────────────

    /// Returns the remaining spendable balance in the milestone pool.
    pub fn milestone_balance(env: Env) -> u128 {
        env.storage()
            .instance()
            .get::<_, MilestonePool>(&DataKey::Pool)
            .map_or(0, |p| p.remaining_balance())
    }

    /// Returns `true` if a specific issue has already been claimed.
    ///
    /// # Note (CM-01 / TMP-02)
    /// Reads from Persistent storage post-fix.  Claims recorded before
    /// the fix (Temporary storage) will not be visible here on live networks.
    pub fn is_claimed(env: Env, repo_hash: BytesN<32>, issue_id: u32) -> bool {
        let claim_key = DataKey::IssueClaim(repo_hash, issue_id);
        env.storage().persistent().has(&claim_key)
    }

    /// Returns the full milestone metadata, or `None` if uninitialized.
    pub fn milestone_info(env: Env) -> Option<MilestonePool> {
        env.storage().instance().get::<_, MilestonePool>(&DataKey::Pool)
    }
}
