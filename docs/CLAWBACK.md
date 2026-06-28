# Clawback: Timing & Use Cases

## Overview

`clawback_expired_funds` returns unclaimed escrowed funds to the pool maintainer after the milestone expiry. This document explains when and how clawback works, and the scenarios it supports.

## Timing

### Expiry Mechanism

- Every pool has an immutable `expiry` timestamp (Unix seconds, ledger time) set at creation.
- Clawback is only callable when `env.ledger().timestamp() >= pool.expiry`.
- The expiry is validated at creation (`expiry > now`) to prevent a zero or past expiry from being stored.
- Ledger timestamps are set by the Stellar network — they cannot be manipulated by the caller.

### When Clawback Becomes Available

1. **Before expiry**: All `clawback_expired_funds` calls revert with `PoolNotExpired`.
2. **At or after expiry**: The maintainer may reclaim the remaining balance at any time.
3. **After successful clawback**: The pool is effectively closed (`total_funds = allocated_funds`, remaining balance = 0). Subsequent clawback calls revert with `NoFundsToClawback`.

### Irreversibility

Once clawback executes, **any unclaimed issue bounties become permanently unclaimable**. There is no mechanism to re-fund or re-open a clawed-back pool. Maintainers must ensure all legitimate claims are settled before triggering clawback.

## Use Cases

### 1. Milestone Completed with Unclaimed Bounties

A milestone reaches its deadline. Most issues were completed and paid, but some were never resolved. The maintainer claws back the remaining funds.

**Flow**:
- Several `release_issue_bounty` calls pay out completed issues.
- Milestone expiry passes.
- `clawback_expired_funds` returns the surplus to the maintainer.

### 2. Milestone Abandoned

A project is cancelled mid-milestone. No issues (or only a few) were completed. The maintainer reclaims the entire unspent pool.

**Flow**:
- Pool is created and funded.
- Few or no bounties are released.
- Expiry passes.
- `clawback_expired_funds` returns the full balance to the maintainer.

### 3. All Bounties Fully Claimed

All milestone issues were completed and paid. The pool has zero remaining balance. Clawback is not possible — the contract rejects the call with `NoFundsToClawback`. This is correct: there are no funds to reclaim.

### 4. Empty Pool (Post-Clawback)

After a successful clawback, the pool's remaining balance is zero. Calling `clawback_expired_funds` again returns `NoFundsToClawback`. This prevents double-dipping and ensures accounting integrity.

## Best Practices

### Setting the Expiry Window

- Choose an expiry well beyond the expected milestone end date (recommended: **+30 days minimum**).
- Account for delays in issue resolution, review cycles, and off-chain coordination.
- The expiry cannot be extended after pool creation.

### Monitoring

- Track `FundsClawedBackEvent` events off-chain to record when clawbacks occur.
- Monitor the pool's `milestone_balance` to know how much remains claimable.
- Alert maintainers when expiry is approaching if unclaimed bounties remain.

### Off-Chain Coordination

- Notify contributors before triggering clawback so they can submit any pending claims.
- For large milestones, consider setting multiple pools with staggered expiries rather than one monolithic pool.

## Security Considerations

### Authorization Isolation

Clawback uses **direct address equality** (`maintainer == pool.maintainer`) rather than consulting WaveGuard. This deliberately isolates the clawback path from a potential WaveGuard compromise — even if the registry is compromised, an attacker cannot reroute clawed-back funds.

### No Double-Clawback

After a successful clawback, `pool.total_funds` is set to `pool.allocated_funds`, making `remaining_balance()` return zero. Any subsequent clawback attempt fails with `NoFundsToClawback`. This is verified in both unit tests (`test_double_clawback_rejected`) and integration tests (`test_clawback_on_empty_pool`).

### Premature Clawback Prevention

The `now >= pool.expiry` check prevents any caller from draining the pool before the milestone deadline. This is the primary safeguard for contributors relying on the escrow.

## Testing Coverage

| Test | File | Scenario |
|------|------|----------|
| `test_clawback_event_emitted` | `tests/clawback.rs` | Verifies `FundsClawedBackEvent` is emitted with correct maintainer and amount |
| `test_clawback_expired_funds` | `src/test.rs` | Happy path: partial claims, clawback returns remainder |
| `test_clawback_full_remaining_after_partial_claims` | `tests/clawback.rs` | Single claim, then full clawback |
| `test_clawback_full_pool_no_claims` | `tests/clawback.rs` | No claims, clawback returns entire pool |
| `test_clawback_after_all_bounties_claimed` | `tests/clawback.rs` | Multiple claims, clawback returns surplus |
| `test_clawback_on_empty_pool` | `tests/clawback.rs` | All funds claimed, clawback rejected |
| `test_clawback_when_pool_empty_rejected` | `tests/clawback.rs` | Pool drained in one claim, clawback rejected |
| `test_double_clawback_rejected` | `src/test.rs` | Second clawback after successful first |
| `test_clawback_before_expiry_rejected` | `tests/clawback.rs` | Clawback called before expiry |
| `test_clawback_non_maintainer_rejected` | `tests/clawback.rs` | Stranger calls clawback |
| `test_clawback_pool_not_found` | `tests/clawback.rs` | Clawback with no pool exists |
| `test_revoked_maintainer_can_still_clawback_own_funds` | `src/test.rs` | Revoked maintainer still owns their pool |
| `test_rogue_co_maintainer_cannot_clawback_others_pool` | `src/test.rs` | Co-maintainer cannot clawback another's pool |
| `test_no_release_after_clawback_drains_pool` | `src/test.rs` | No releases after clawback |
