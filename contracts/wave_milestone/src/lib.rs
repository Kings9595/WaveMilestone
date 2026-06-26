#![no_std]

mod events;
mod test;
pub mod types;

use events::{
    BountyReleasedEvent, FundsClawedBackEvent, PoolCreatedEvent, TOPIC_BOUNTY_RELEASED,
    TOPIC_FUNDS_CLAWED_BACK, TOPIC_POOL_CREATED,
};
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Symbol};

use types::{DataKey, Error, IssueClaim, MilestonePool, TokenClient, WaveGuardClient};

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
/// Original code stored `IssueClaim` in **Temporary** storage.  Stellar's
/// Temporary storage entries are pruned after their TTL expires.  Once pruned,
/// `env.storage().temporary().get(...)` returns `None`, the duplicate-claim
/// guard treats the issue as unclaimed, and a maintainer can re-release the
/// same bounty.  This has been **fixed** by migrating `IssueClaim` records to
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
/// After CM-01's fix, `IssueClaim` entries now live in Persistent storage.
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
        // ── Authentication ──
        maintainer.require_auth();

        // ── WaveGuard validation ──
        let guard = WaveGuardClient::new(&env, &guard_contract);
        if !guard.is_maintainer(&maintainer) {
            return Err(Error::UnauthorizedMaintainer);
        }

        // ── Input validation ──
        if total_funds == 0 {
            return Err(Error::InvalidAmount);
        }
        let now = env.ledger().timestamp();
        if expiry <= now {
            return Err(Error::ExpiryInPast);
        }

        // ── Fund transfer ──
        let token = TokenClient::new(&env, &asset);
        token.transfer(&maintainer, &env.current_contract_address(), &(total_funds as i128));

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
    /// # Auth
    /// - `maintainer.require_auth()` — the caller must sign.
    /// - WaveGuard `is_maintainer` check passes.
    ///
    /// # Trust Assumptions
    /// - `developer` is caller-supplied and not restricted on-chain.
    ///   A malicious maintainer can direct the bounty to any address.
    ///   Mitigation is governance-layer: WaveGuard revocation (see CM-02).
    ///
    /// # Claim Storage (Security Fix CM-01)
    /// Claim records are stored in **Persistent** storage (not Temporary).
    /// Temporary storage entries expire after their TTL, which would allow
    /// a pruned entry to be re-claimed.  Persistent storage ensures the
    /// duplicate-claim guard is durable for the contract's lifetime.
    pub fn release_issue_bounty(
        env: Env,
        maintainer: Address,
        repo_hash: BytesN<32>,
        issue_id: u32,
        developer: Address,
        amount: u128,
    ) -> Result<(), Error> {
        // ── Authentication ──
        maintainer.require_auth();

        // ── Load pool ──
        let mut pool = env
            .storage()
            .instance()
            .get::<_, MilestonePool>(&DataKey::Pool)
            .ok_or(Error::PoolNotFound)?;

        // ── WaveGuard validation ──
        let guard = WaveGuardClient::new(&env, &pool.guard_contract);
        if !guard.is_maintainer(&maintainer) {
            return Err(Error::UnauthorizedMaintainer);
        }

        // ── Duplicate-claim guard (CM-01: reads Persistent storage) ──
        // SECURITY: Must use Persistent storage here. Temporary storage entries
        // expire after their TTL; a lapsed entry returns None, bypassing this
        // guard and allowing a maintainer to re-claim the same issue bounty.
        let claim_key = DataKey::IssueClaim(repo_hash.clone(), issue_id);
        if env
            .storage()
            .persistent()
            .get::<_, IssueClaim>(&claim_key)
            .is_some_and(|c| c.completed)
        {
            return Err(Error::BountyAlreadyClaimed);
        }

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
        token.transfer(&env.current_contract_address(), &developer, &(amount as i128));

        // ── Update pool state ──
        pool.allocated_funds = pool.allocated_funds.checked_add(amount).ok_or(Error::InvalidAmount)?;
        env.storage().instance().set(&DataKey::Pool, &pool);

        // ── Record claim in Persistent storage (CM-01 fix) ──
        let claim = IssueClaim {
            issue_id,
            developer: developer.clone(),
            payment_amount: amount,
            completed: true,
            maintainer: maintainer.clone(),
            claimed_at: env.ledger().timestamp(),
        };
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
    /// Only callable by the original `pool.maintainer` and only after
    /// `pool.expiry` has passed. Transfers the full remaining balance
    /// back to the maintainer and zeroes out the available pool.
    ///
    /// # Auth
    /// - `maintainer.require_auth()` — the caller must sign.
    /// - `maintainer` must match `pool.maintainer` (address equality check).
    ///
    /// # Trust Assumptions
    /// - Only the address stored as `pool.maintainer` at creation time can
    ///   trigger clawback.  WaveGuard is NOT consulted here — the check is
    ///   a direct address comparison so that a WaveGuard compromise cannot
    ///   reroute funds via this path.
    pub fn clawback_expired_funds(env: Env, maintainer: Address) -> Result<(), Error> {
        maintainer.require_auth();

        let mut pool = env
            .storage()
            .instance()
            .get::<_, MilestonePool>(&DataKey::Pool)
            .ok_or(Error::PoolNotFound)?;

        if maintainer != pool.maintainer {
            return Err(Error::UnauthorizedCaller);
        }

        let now = env.ledger().timestamp();
        if now < pool.expiry {
            return Err(Error::PoolNotExpired);
        }

        let remaining = pool.remaining_balance();
        if remaining == 0 {
            return Err(Error::NoFundsToClawback);
        }

        let token = TokenClient::new(&env, &pool.asset);
        token.transfer(&env.current_contract_address(), &maintainer, &(remaining as i128));

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
        env.storage().persistent().get::<_, IssueClaim>(&claim_key).is_some_and(|c| c.completed)
    }

    /// Returns the full milestone metadata, or `None` if uninitialized.
    pub fn milestone_info(env: Env) -> Option<MilestonePool> {
        env.storage().instance().get::<_, MilestonePool>(&DataKey::Pool)
    }
}
