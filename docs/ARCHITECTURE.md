# WaveMilestone Architecture

## System Overview

WaveMilestone is a Stellar Soroban smart contract that implements an automated milestone escrow vault. It links a GitHub Milestone budget to on-chain micro-payouts that are released as issues are completed.

```
┌──────────────────────────────────────────────────────────┐
│                     Off-Chain                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │ GitHub       │  │ Maintainer  │  │ Contributor     │  │
│  │ Milestone    │  │ (Wallet)    │  │ (Wallet)        │  │
│  └──────┬───────┘  └──────┬──────┘  └────────┬────────┘  │
│         │                 │                   │           │
└─────────┼─────────────────┼───────────────────┼───────────┘
          │            TX    │                   │
          ▼                 ▼                    ▼
┌──────────────────────────────────────────────────────────┐
│                     Stellar Network                       │
│  ┌──────────────────────────────────────────────────────┐│
│  │              WaveMilestone Contract                   ││
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ ││
│  │  │   Instance    │  │  Persistent  │  │   Events   │ ││
│  │  │   Storage     │  │   Storage    │  │   Emitter  │ ││
│  │  │  (Pool Meta)  │  │(ClaimRecord) │  │            │ ││
│  │  └──────────────┘  └──────────────┘  └────────────┘ ││
│  └───────────────────────┬──────────────────────────────┘│
│                          │                               │
│  ┌───────────────────────▼──────────────────────────────┐│
│  │              WaveGuard Contract                       ││
│  │           (Access Registry / Auth)                    ││
│  └───────────────────────┬──────────────────────────────┘│
│                          │                               │
│  ┌───────────────────────▼──────────────────────────────┐│
│  │           Stellar Asset Contract (SAC)                ││
│  │              (Token Transfers)                        ││
│  └──────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────┘
```

## Smart Contract Architecture

### Contract Composition

The `WaveMilestoneContract` is a single Soroban contract with three public lifecycle methods and three view methods:

| Method | Category | Description |
|--------|----------|-------------|
| `create_milestone_pool` | Lifecycle | Initialize escrow vault, lock funds, set expiry |
| `release_issue_bounty` | Lifecycle | Release micro-payout per completed issue |
| `clawback_expired_funds` | Lifecycle | Return unclaimed funds after expiry |
| `milestone_balance` | View | Query remaining pool balance |
| `is_claimed` | View | Check if an issue was already paid |
| `milestone_info` | View | Get full pool metadata |

### Cross-Contract Dependencies

1. **WaveGuard** (`guard_contract`)
   - Interface method: `is_maintainer(address) -> bool`
   - Called on every `create_milestone_pool` and `release_issue_bounty` invocation.

2. **Stellar Asset Contract (SAC)** (`asset`)
   - Interface method: `transfer(from, to, amount)`
   - Called during pool creation (funding) and bounty release (payout).

## Storage Architecture

### Instance Storage (Persistent, Contract Lifetime)

```rust
DataKey::Pool -> MilestonePool {
    guard_contract: Address,
    asset: Address,
    total_funds: u128,
    allocated_funds: u128,
    expiry: u64,
    maintainer: Address,
}
```

- **Bumped on every write** (create_pool, release_bounty, clawback).
- Stores aggregate pool state; read-heavy access for view methods.

### Persistent Storage — Claim Records

```rust
DataKey::IssueClaim(BytesN<32>, u32) -> ClaimRecord {
    payment_amount: u128,
    completed: bool,
}
```

Claim records are stored in **Persistent** storage (not Temporary).  A prior
design used Temporary storage, but entries there are pruned after their TTL
(~1 month on Mainnet).  Once pruned, the duplicate-claim guard would see `None`
and allow the same `(repo_hash, issue_id)` pair to be re-released — a critical
drain vulnerability (see security finding CM-01).  Persistent storage makes the
guard durable for the contract's lifetime.

The key encodes both `repo_hash` and `issue_id`, so only `payment_amount` and
`completed` need to be stored in the record itself — `issue_id` and `developer`
are available from the call context and the storage key respectively.

### Why This Split?

| Criteria | Instance (`Pool`) | Persistent (`ClaimRecord`) |
|----------|-------------------|-----------------------------|
| Lifetime | Contract lifetime | Contract lifetime |
| Read frequency | High (view methods) | Low (only on release/query) |
| Update frequency | Medium (per claim) | Never (write-once) |
| Data criticality | Pool integrity | Duplicate-claim prevention |

### Claim Record Cleanup

`ClaimRecord` entries in Persistent storage accumulate over the contract's
lifetime — one entry per successfully released issue.  Unlike Temporary
storage, Persistent entries are not automatically pruned.

**Guidelines for operators:**

1. **Ledger TTL rent**: Persistent entries require periodic ledger-fee bumps to
   stay active.  The contract bumps the Instance storage entry on every write,
   but individual `ClaimRecord` keys under `DataKey::IssueClaim` are **not**
   automatically extended by contract logic.  On Mainnet, entries whose rent
   is not extended will eventually be archived (moved to the historical
   ledger), which has the same effect as deletion and would re-open the
   duplicate-claim guard for that key.

   **Mitigation**: Either (a) issue periodic `extend_ttl` calls on all
   `ClaimRecord` keys via an off-chain maintenance script, or (b) accept that
   the practical claim window matches the TTL and document this as an
   operational constraint.  The current implementation relies on (b); issue
   bounty windows are typically shorter than the Persistent entry TTL.

2. **No manual deletion path**: There is no contract method to delete a
   `ClaimRecord`.  This is intentional — deletion would reopen replay
   protection for that key.  If a record must be invalidated for operational
   reasons (e.g. incorrectly issued claim), a contract upgrade is required.

3. **Off-chain indexing**: To enumerate all claim records, subscribe to
   `BountyReleased` events from the contract's event stream.  On-chain
   iteration over storage keys is not supported in Soroban; the event log is
   the canonical index of all claims.


## Authentication & Authorization

### Dual Validation Flow

```
Client TX ──► maintainer.require_auth() ──► WaveGuard.is_maintainer()
                    │
                    ├── Signature verified  ──► Pass
                    ├── Signature invalid   ──► Revert
                    │
                    ▼
            WaveGuard check
                    │
                    ├── Registered maintainer ──► Authorized
                    ├── Unregistered         ──► UnauthorizedMaintainer
                    │
                    ▼
            Clawback only: caller == pool.maintainer
                    │
                    ├── Match  ──► Authorized
                    ├── No match ──► UnauthorizedCaller
```

1. **Transaction-level auth**: `Address::require_auth()` ensures the transaction is signed by the claimed maintainer.
2. **Registry-level auth**: WaveGuard cross-contract call verifies the signer is an active, non-revoked maintainer.
3. **Pool-level auth** (clawback only): The clawback caller must match the `pool.maintainer` exactly.

## Security Properties

### Duplicate Claim Prevention

- Storage key: `DataKey::IssueClaim(repo_hash, issue_id)` — composite of repo identity and issue number.
- Once `completed == true` in the `ClaimRecord`, all subsequent `release_issue_bounty` calls with the same key revert with `BountyAlreadyClaimed`.
- This prevents drain attacks via replay of claim transactions.

### Balance Overflow Protection

- Every `release_issue_bounty` checks `amount <= pool.remaining_balance()` before any transfer.
- If the check fails, the transaction reverts with `InsufficientPoolBalance` — no tokens are moved, pool state is unchanged.
- This prevents accidental or malicious over-allocation from locking remaining funds.

### Maintainer Revocation

- WaveGuard is the single source of truth for maintainer identity.
- If a maintainer is removed from WaveGuard mid-milestone, all subsequent `release_issue_bounty` calls from that address revert with `UnauthorizedMaintainer`.
- Already-claimed bounties are unaffected (finality is preserved).

## Data Flow: Full Lifecycle

```
1. SETUP PHASE
   Maintainer ──► Deploy WaveMilestone + WaveGuard
               ──► Register as maintainer in WaveGuard
               ──► Mint/lock funds

2. POOL CREATION
   Maintainer ──► create_milestone_pool(guard, asset, total_funds, expiry)
               │
               ├── require_auth()
               ├── WaveGuard.is_maintainer() ✓
               ├── Token.transfer(maintainer → contract, total_funds)
               ├── Storage: Pool { total_funds, allocated_funds: 0, ... }
               └── Event: MilestonePoolCreated

3. BOUNTY RELEASE (per issue)
   Maintainer ──► release_issue_bounty(repo_hash, issue_id, developer, amount)
               │
               ├── require_auth()
               ├── WaveGuard.is_maintainer() ✓
               ├── Storage: Key(issue_id).completed == false
               ├── amount <= remaining_balance() ✓
               ├── Token.transfer(contract → developer, amount)
               ├── Storage: Pool.allocated_funds += amount
               ├── Storage: IssueClaim { completed: true }
               └── Event: BountyReleased

4. CLAWBACK (after expiry)
   Maintainer ──► clawback_expired_funds()
               │
               ├── require_auth()
               ├── caller == pool.maintainer ✓
               ├── now >= pool.expiry ✓
               ├── remaining > 0 ✓
               ├── Token.transfer(contract → maintainer, remaining)
               ├── Storage: Pool.total_funds = Pool.allocated_funds
               └── Event: FundsClawedBack
```

## Testing Architecture

### Test Layers

| Layer | Location | Scope |
|-------|----------|-------|
| Unit tests | `src/test.rs` | Individual function correctness, edge cases |
| Integration tests | `tests/*.rs` | Cross-contract interactions, lifecycle scenarios |
| Mock contracts | `tests/common/` | MockToken, MockWaveGuard for deterministic testing |

### Mock Contracts

**MockToken**: Simulates SAC token behavior with in-storage balance tracking. Supports `mint`, `transfer`, `balance` — enough for full lifecycle testing.

**MockWaveGuard**: Simple boolean registry. Supports `add_maintainer`, `remove_maintainer`, `is_maintainer` — enables testing of access control scenarios.

### Test Scenarios

See [README](../README.md#testing) for the full matrix of test scenarios.
