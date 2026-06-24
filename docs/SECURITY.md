# WaveMilestone Security

## Threat Model

WaveMilestone is a smart contract that holds real assets in escrow. The primary security goals are:

1. **Fund Safety**: No one except authorized maintainers can move funds out of the contract.
2. **Fair Distribution**: Each issue can be claimed exactly once — no double-spend.
3. **Timely Clawback**: Unclaimed funds are returnable to the maintainer after milestone expiry.
4. **Access Control**: Only verified maintainers can trigger payouts.

## Vulnerability Analysis

### 1. Duplicate Claim / Drain Attack

**Risk**: An attacker replays a `release_issue_bounty` transaction to drain the pool.

**Mitigation**:
- Composite storage key `(repo_hash, issue_id)` ensures uniqueness across repositories.
- Once `completed` is set to `true`, any subsequent `release_issue_bounty` with the same key reverts with `BountyAlreadyClaimed`.
- The check occurs *before* any token transfer — even a race condition cannot result in double payment.

### 2. Over-Allocation / Pool Insolvency

**Risk**: A maintainer (or attacker controlling a maintainer key) attempts to allocate more than the pool balance.

**Mitigation**:
- `amount > pool.remaining_balance()` check guards every `release_issue_bounty` call.
- On failure, the entire transaction reverts — no partial state change, no locked funds.
- `remaining_balance()` is computed as `total_funds - allocated_funds` (checked arithmetic via Soroban's overflow protection).

### 3. Unauthorized Maintainer

**Risk**: An attacker without maintainer privileges calls lifecycle methods.

**Mitigation**:
- **Dual validation**: `Address::require_auth()` (cryptographic signature verification) + WaveGuard `is_maintainer()` (registry lookup).
- Short of compromising the maintainer's Stellar private key **and** manipulating the WaveGuard registry, this attack is infeasible.
- WaveGuard can revoke a maintainer at any time; revoked maintainers immediately lose access.

### 4. Premature Clawback

**Risk**: A maintainer drains the pool before issues are settled.

**Mitigation**:
- `clawback_expired_funds` checks `now >= pool.expiry` before allowing any transfer.
- Only the `pool.maintainer` (recorded at pool creation) can call clawback — even other registered maintainers cannot.
- The expiry is set at pool creation and is immutable for the pool's lifetime.

### 5. Transfer Failure Path Coverage

**Risk**: A token transfer reverts mid-execution (e.g., the SAC contract panics or the contract's token balance is unexpectedly zero), leaving the contract in an inconsistent state — storage marked as paid but no tokens delivered, or vice versa.

**Mitigation**:
- The contract follows strict **check-effects-interaction** ordering in every payout path:
  1. All inputs and auth validated (checks).
  2. `pool.allocated_funds` incremented and `IssueClaim.completed` set to `true` (effects).
  3. `token.transfer(...)` called last (interaction).
- If `token.transfer(...)` panics, Soroban's host rolls back the entire transaction — storage writes are also reverted, so no funds are marked paid without being delivered.
- The `TransferFailed` error variant (`Error::TransferFailed = 8`) is reserved for callers and wrappers that need to surface a transfer-specific error; the Soroban host itself guarantees atomicity at the host level.
- `clawback_expired_funds` follows the same ordering: state is zeroed before the transfer, and a host-level panic reverts both atomically.

**Audit verification**:
- Confirm `token.transfer(...)` is always the **last** statement in `release_issue_bounty` and `clawback_expired_funds`.
- Confirm no storage write occurs after a `token.transfer(...)` call.
- Confirm `Error::TransferFailed` is not silently swallowed anywhere in calling code.

### 6. Expiry-Based Clawback Security

**Risk**: Incorrect expiry handling allows either (a) a maintainer to reclaim funds prematurely while issues are still open, or (b) funds to be permanently locked if the expiry check contains an off-by-one or is bypassable.

**Mitigation**:
- `clawback_expired_funds` enforces a strict `now >= pool.expiry` check using `env.ledger().timestamp()`, which is the Soroban-provided ledger close time — it cannot be manipulated by the caller.
- Only the address recorded as `pool.maintainer` at pool creation can trigger clawback; other registered maintainers are explicitly rejected (`Error::UnauthorizedCaller`).
- The expiry timestamp is validated at pool creation (`expiry > now`) so a zero or past expiry cannot be stored.
- Once clawback succeeds, `pool.total_funds` is set to `pool.allocated_funds`, effectively closing the pool with a remaining balance of zero — future clawback calls return `Error::NoFundsToClawback` rather than re-entering the transfer path.
- Contributors should be aware that **any unclaimed issue bounties become unclaimable once clawback has executed**. Maintainers must ensure all legitimate claims are settled before the expiry deadline, or use a sufficiently long expiry window (recommend at least 30 days beyond the milestone end date).

**Audit verification**:
- Confirm `now >= pool.expiry` (not `>`) to include the exact expiry ledger second.
- Confirm only `pool.maintainer` — not any WaveGuard-registered maintainer — can call clawback.
- Confirm `pool.expiry > now` is enforced at creation time.
- Confirm post-clawback pool state prevents double-clawback.

### 7. Reentrancy

**Risk**: A malicious token contract calls back into WaveMilestone during `transfer()`.

**Mitigation**:
- The contract follows the **check-effects-interaction** pattern:
  1. Validate inputs and auth (✓)
  2. Update storage state (✓)
  3. Emit events (✓)
  4. Transfer tokens (last)
- Token transfers are the final operation. Any reentrant call would see already-updated state and cannot claim the same issue twice.
- Soroban's environment provides additional reentrancy protection at the host level.

### 8. Front-Running

**Risk**: An attacker observes a pending `release_issue_bounty` and front-runs it with a higher gas payment to claim the bounty for themselves.

**Mitigation**:
- The `developer` address is specified by the maintainer in the transaction parameters.
- Even if an attacker front-runs the transaction, they would be sending funds to the same `developer` address (not to themselves).
- The attacker gains nothing by front-running.

## Audit Checklist

Items to verify during security review:

- [ ] `require_auth()` is called on every public method before state mutation.
- [ ] WaveGuard `is_maintainer()` check cannot be bypassed.
- [ ] Composite `(repo_hash, issue_id)` key prevents cross-repo collisions.
- [ ] `BountyAlreadyClaimed` error is emitted before any token transfer.
- [ ] `InsufficientPoolBalance` error reverts entirely — no partial state changes.
- [ ] `clawback_expired_funds` enforces `now >= pool.expiry`.
- [ ] Only `pool.maintainer` can clawback (not any registered maintainer).
- [ ] All arithmetic is checked (Soroban overflow protection enabled in release profile).
- [ ] Events are emitted for all state-changing operations.
- [ ] Zero-amount operations are rejected.
- [ ] Past expiry timestamps are rejected at pool creation.

## Responsible Disclosure

If you discover a security vulnerability in WaveMilestone:

1. **Do not** open a public GitHub issue.
2. **Do not** post details in public forums or discussions.
3. Email the maintainers directly or contact via the [security advisory process](https://github.com/anomalyco/wave-milestone/security/advisories).
4. Provide full details of the vulnerability, including reproduction steps and potential impact.
5. Allow a reasonable period (90 days) for a fix before any public disclosure.

We take all security reports seriously and will acknowledge receipt within 48 hours.

## Bug Bounty

A bug bounty program may be established for high-severity vulnerabilities affecting mainnet deployments. Check the repository discussions for current programs.

## Security-Related Configuration

### Release Profile (Cargo.toml)

```toml
[profile.release]
opt-level = "z"           # Optimize for size
overflow-checks = true    # Runtime integer overflow protection
debug = 0                 # Strip debug symbols
strip = "symbols"         # Strip all symbols
lto = true                # Link-time optimization
codegen-units = 1         # Maximize optimization surface
panic = "abort"           # No unwinding in production
```

### Recommended Maintainer Setup

- Use a **hardware wallet** or **multisig** for the maintainer key on mainnet.
- Rotate maintainer keys periodically via WaveGuard.
- Monitor the contract's `milestone_balance` and event logs regularly.
- Set `expiry` with a generous buffer (e.g., +30 days past the expected milestone end).
