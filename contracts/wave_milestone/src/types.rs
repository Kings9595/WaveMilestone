use soroban_sdk::{contractclient, contracterror, contracttype, Address, BytesN, Env};

// ─────────────────────────────────────────────────────────────
// Data Structures
// ─────────────────────────────────────────────────────────────

/// Core escrow pool representing a funded milestone.
///
/// Stored in **Instance** storage for the contract's lifetime.
///
/// # Trust Assumptions
/// - `guard_contract`: Address of the WaveGuard registry consulted for all
///   maintainer-privileged operations.  This value is immutable after pool
///   creation.  A compromised WaveGuard contract at this address can grant
///   arbitrary maintainer status and drain the pool.
/// - `maintainer`: The original pool creator.  Used as the sole authorized
///   caller for `clawback_expired_funds` (direct address equality — WaveGuard
///   is NOT re-checked on clawback to isolate the clawback path from a
///   potential WaveGuard compromise).
/// - `asset`: Must be a trusted Stellar Asset Contract (SAC).  A malicious
///   token could re-enter this contract during `transfer` calls or silently
///   fail, leaving pool accounting out of sync with actual token balances.
#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilestonePool {
    pub guard_contract: Address,
    pub asset: Address,
    pub total_funds: u128,
    pub allocated_funds: u128,
    pub expiry: u64,
    pub maintainer: Address,
}

impl MilestonePool {
    /// Returns the unallocated amount remaining in the pool.
    ///
    /// Uses saturating subtraction to guard against any accounting
    /// inconsistency that would otherwise cause an underflow panic.
    #[must_use]
    pub fn remaining_balance(&self) -> u128 {
        self.total_funds.saturating_sub(self.allocated_funds)
    }
}

/// Individual issue bounty claim record.
///
/// # Storage Note (Security — CM-01)
/// Stored in **Persistent** storage.  A previous version used Temporary
/// storage, whose entries expire after a ledger TTL.  Once a Temporary entry
/// is pruned, the duplicate-claim guard returns `None` for that key, allowing
/// the same `(repo_hash, issue_id)` pair to be re-claimed.  Persistent storage
/// ensures the completed flag survives for the contract's lifetime, making the
/// duplicate-claim guard durable.
///
/// ## Temporary Storage Leakage Risk (TMP-01)
/// Any future use of Temporary storage for authorization state — such as
/// one-time nonces, session tokens, or flags — is subject to the same
/// expiry-based re-use attack unless the TTL is explicitly managed and
/// checked.  Authorization-critical state MUST use Instance or Persistent
/// storage.
#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueClaim {
    pub issue_id: u32,
    pub developer: Address,
    pub payment_amount: u128,
    pub completed: bool,
}

// ─────────────────────────────────────────────────────────────
// Storage Keys
// ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    /// Singleton — the active milestone pool (Instance storage)
    Pool,
    /// Per-issue claim under a specific repository (Persistent storage)
    ///
    /// Key: (repo_hash: BytesN<32>, issue_id: u32)
    ///
    /// SECURITY: This key MUST be read/written via `persistent()` storage.
    /// Using `temporary()` for this key bypasses the duplicate-claim guard
    /// after TTL expiry (see CM-01 in lib.rs).
    IssueClaim(BytesN<32>, u32),
}

// ─────────────────────────────────────────────────────────────
// Error Enum
// ─────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    PoolNotFound = 1,
    PoolNotExpired = 2,
    BountyAlreadyClaimed = 3,
    InsufficientPoolBalance = 4,
    UnauthorizedMaintainer = 5,
    UnauthorizedCaller = 6,
    NoFundsToClawback = 7,
    TransferFailed = 8,
    InvalidAmount = 9,
    ExpiryInPast = 10,
}

// ─────────────────────────────────────────────────────────────
// Cross-Contract Interfaces
// ─────────────────────────────────────────────────────────────

/// WaveGuard identity registry interface.
///
/// Verifies that an address is an authorized maintainer.
///
/// # Trust Assumption
/// The contract at this address is the single point of authority for
/// maintainer identity.  All maintainer-privileged calls (`create_milestone_pool`,
/// `release_issue_bounty`) rely on the truthfulness of `is_maintainer`.
/// Deployers must ensure WaveGuard is not upgradeable by an untrusted party.
#[contractclient(name = "WaveGuardClient")]
pub trait WaveGuardInterface {
    fn is_maintainer(env: Env, address: Address) -> bool;
}

/// Standard Stellar Asset Contract (SAC) token interface.
///
/// # Trust Assumption
/// The token at `pool.asset` is trusted to:
/// - Execute transfers atomically without re-entering this contract.
/// - Report accurate balances.
/// - Not silently absorb or lose funds during transfer.
///
/// Deployment must use only verified SAC instances.
#[contractclient(name = "TokenClient")]
pub trait TokenInterface {
    fn transfer(env: Env, from: Address, to: Address, amount: u128);
    fn balance(env: Env, id: Address) -> u128;
}
