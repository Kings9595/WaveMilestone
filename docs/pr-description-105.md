# Add Guard Rails for Invalid repo_hash Values

## Summary

Adds input validation to `release_issue_bounty` that rejects an all-zero
`repo_hash` (`[0u8; 32]`) before any storage or cross-contract call is made.
An all-zero 32-byte hash is the canonical unset/null value and is not a valid
SHA-256 repository identifier. Passing it previously allowed claim records to
be stored under a meaningless key, polluting persistent storage and potentially
confusing duplicate-claim detection.

---

## Problem

`release_issue_bounty` accepted any `BytesN<32>` as `repo_hash` with no
validation. A caller (or bug) could pass an all-zero hash and:

- Store a `IssueClaim` record under the null key in Persistent storage.
- Block a legitimate claim for `(zero_hash, issue_id)` forever (the
  duplicate-claim guard would fire on the second attempt).
- Make storage state ambiguous — it is impossible to tell whether a
  `(zero_hash, issue_id)` entry was intentionally created or is the result
  of a misconfigured client.

The same logic applies to `clawback_expired_funds`, but that function takes no
`repo_hash` parameter. The only entry point that accepts `repo_hash` as input
is `release_issue_bounty`, so that is the only site that needs guarding.

---

## Changes

### `contracts/wave_milestone/src/types.rs`

Added `InvalidRepoHash = 11` to the `Error` enum:

```rust
pub enum Error {
    // ...existing variants...
    InvalidAmount = 9,
    ExpiryInPast = 10,
    InvalidRepoHash = 11,   // ← new
}
```

### `contracts/wave_milestone/src/lib.rs`

Added a guard immediately after `maintainer.require_auth()` in
`release_issue_bounty`, before any pool load or storage access:

```rust
// ── repo_hash validation ──
if repo_hash == BytesN::from_array(&env, &[0u8; 32]) {
    return Err(Error::InvalidRepoHash);
}
```

Placing the check before the pool load means no cross-contract call (WaveGuard)
or storage read is performed when the input is invalid — a minor but clean
fail-fast improvement.

### `contracts/wave_milestone/src/test.rs`

- Fixed `setup()`: `repo_hash` changed from `[0u8; 32]` to `[1u8; 32]` so
  all existing unit tests continue to pass under the new validation.
- Fixed `test_multiple_issues_different_repos_independent`: `repo_b` changed
  from `[1u8; 32]` to `[2u8; 32]` to avoid collision with the updated
  `t.repo_hash`.
- Added two new tests:

| Test | What it verifies |
|---|---|
| `test_release_bounty_rejects_zero_repo_hash` | `[0u8; 32]` → `Error::InvalidRepoHash` |
| `test_release_bounty_accepts_nonzero_repo_hash` | `[1u8; 32]` → succeeds, claim recorded |

### `contracts/wave_milestone/tests/common/mod.rs`

- `repo_hash` changed from `[0u8; 32]` to `[1u8; 32]`.
- `repo_hash_two` changed from `[1u8; 32]` to `[2u8; 32]`.

All integration tests that use `TestContext` continue to pass without
modification because they access `repo_hash` through the context struct.

---

## Validation Rule

| Input | Result |
|---|---|
| `repo_hash == [0u8; 32]` | `Err(Error::InvalidRepoHash)` |
| Any other 32-byte value | Proceeds normally |

The check is intentionally minimal — only the all-zero hash is rejected.
Non-zero hashes that do not correspond to a real GitHub repository are an
off-chain concern; the contract's role is to reject values that are
structurally invalid at the encoding level.

---

closes #105
