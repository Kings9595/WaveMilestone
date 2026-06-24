# WaveMilestone Contract Release Audit Checklist

This checklist must be completed before any contract release to production/mainnet.

---

## 1. Pre-Audit Preparation

- [ ] **Code Freeze**: No new features merged after audit start date
- [ ] **Version Tagged**: Release candidate tagged (e.g., `v1.0.0-rc.1`)
- [ ] **Dependencies Locked**: `Cargo.lock` committed and verified
- [ ] **Documentation Current**: All public APIs documented with `///` comments
- [ ] **Changelog Updated**: `CHANGELOG.md` reflects all changes since last release

---

## 2. Static Analysis & Tooling

- [ ] `cargo check` passes with zero warnings
- [ ] `cargo clippy -- -D warnings` passes (deny warnings)
- [ ] `cargo fmt -- --check` passes
- [ ] `cargo test` passes (all unit and integration tests)
- [ ] `cargo audit` passes (no vulnerable dependencies)
- [ ] `soroban contract optimize` builds successfully
- [ ] Contract size within limits (< 100KB WASM for mainnet)

---

## 3. Functional Correctness

### Pool Creation (`create_milestone_pool`)
- [ ] Maintainer authorization enforced
- [ ] WaveGuard maintainer validation works
- [ ] Input validation: `total_funds > 0`
- [ ] Input validation: `expiry > now`
- [ ] Token transfer from maintainer to contract succeeds
- [ ] Pool state persisted correctly
- [ ] `PoolCreatedEvent` emitted with correct data

### Bounty Release (`release_issue_bounty`)
- [ ] Maintainer authorization enforced
- [ ] WaveGuard maintainer validation works
- [ ] Duplicate claim prevention: `(repo_hash, issue_id)` uniqueness
- [ ] Balance check: `amount <= remaining_balance`
- [ ] Input validation: `amount > 0`
- [ ] Token transfer from contract to developer succeeds
- [ ] Pool `allocated_funds` updated correctly
- [ ] Claim recorded in temporary storage
- [ ] `BountyReleasedEvent` emitted with correct data

### Clawback (`clawback_expired_funds`)
- [ ] Only pool maintainer can call
- [ ] Expiry check: `now >= pool.expiry`
- [ ] Remaining balance > 0 check
- [ ] Token transfer from contract to maintainer succeeds
- [ ] Pool state updated: `total_funds = allocated_funds`
- [ ] `FundsClawedBackEvent` emitted with correct data

### View Methods
- [ ] `milestone_balance` returns correct remaining balance
- [ ] `is_claimed` returns correct claim status
- [ ] `milestone_info` returns full pool or `None`

---

## 4. Security Review

### Access Control
- [ ] All state-mutating functions require `require_auth()`
- [ ] WaveGuard `is_maintainer` checked on all privileged operations
- [ ] No unauthorized caller can modify pool state
- [ ] Clawback restricted to original maintainer only

### Reentrancy Protection
- [ ] No external calls before state updates (checks-effects-interactions)
- [ ] Token transfers are last operation in each function
- [ ] No callback hooks in token interface that could re-enter

### Integer Safety
- [ ] All arithmetic uses `saturating_*` or checked operations
- [ ] No overflow/underflow possible in `remaining_balance()`
- [ ] `allocated_funds` never exceeds `total_funds`

### State Consistency
- [ ] Pool state atomic updates (no partial writes)
- [ ] Temporary storage for claims (auto-expiry acceptable)
- [ ] No stale reads possible

### Economic Security
- [ ] No fund lockup without clawback path
- [ ] Expiry timestamp enforced
- [ ] Minimum/maximum bounds on amounts (if applicable)
- [ ] No griefing vectors (e.g., spam claims)

---

## 5. Cross-Contract Interactions

### WaveGuard Interface
- [ ] `is_maintainer` called correctly
- [ ] Contract address validated (not spoofed)
- [ ] Error handling for failed calls

### Token Interface (SAC)
- [ ] `transfer` called with correct `from`/`to`/`amount`
- [ ] Return values checked (if token returns status)
- [ ] Compatible with standard Stellar Asset Contract

---

## 6. Event & Indexing Verification

- [ ] All events use `symbol_short!` topics
- [ ] Event structs match `events.rs` definitions
- [ ] Events emitted for all state changes
- [ ] Event data sufficient for off-chain indexing
- [ ] Topic constants documented and consistent

---

## 7. Storage & Upgradeability

- [ ] `DataKey` enum stable (no variant removal)
- [ ] Storage layout compatible with previous version (if upgrading)
- [ ] Instance storage used for singleton pool
- [ ] Temporary storage used for claims (TTL acceptable)
- [ ] No persistent storage bloat

---

## 8. Test Coverage

- [ ] Unit tests for all public functions
- [ ] Integration tests for full lifecycle
- [ ] Edge case tests: zero amounts, expired pools, duplicate claims
- [ ] Unauthorized access tests
- [ ] Over-allocation prevention tests
- [ ] Fuzzing/property tests for arithmetic (if applicable)
- [ ] Coverage target: >90% lines, >80% branches

---

## 9. Deployment Verification

- [ ] Testnet deployment successful
- [ ] Testnet integration test passes (end-to-end)
- [ ] Contract ID recorded
- [ ] WASM hash verified matches source
- [ ] Frontend/integration points tested against deployed contract

---

## 10. Post-Audit Actions

- [ ] All audit findings addressed or accepted with justification
- [ ] `audit:completed` label applied to release PR
- [ ] Audit report archived (link in release notes)
- [ ] Release notes include security-relevant changes
- [ ] Monitoring/alerting configured for mainnet deployment

---

## Sign-Off

| Role | Name | Signature | Date |
|------|------|-----------|------|
| Lead Developer | | | |
| Security Auditor | | | |
| Release Manager | | |

---

**Note**: This checklist is a living document. Update it as the contract evolves and new risks are identified.