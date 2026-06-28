# Add Developer Guide for Testing with Mock Contracts

## Summary

This PR adds `docs/TESTING_WITH_MOCKS.md` — a comprehensive developer guide
that documents how WaveMilestone's test suite uses mock contracts to test
cross-contract interactions. The guide covers the mock implementations, shared
test helpers, all common testing patterns, and step-by-step instructions for
writing new unit and integration tests.

---

## Problem

WaveMilestone calls two external contracts (`WaveGuard` and a Stellar Asset
Contract token) at runtime. New contributors and maintainers working on the
codebase had no written documentation explaining:

- Why mock contracts are used instead of real deployments.
- How `MockWaveGuard` and `MockToken` work and what their design trade-offs are.
- The difference between the inline `TestEnv` in unit tests and the shared
  `TestContext` in integration tests.
- How to use core Soroban test patterns (`env.register`, `mock_all_auths`,
  `Address::generate`, `BytesN::from_array`, `try_*` error variants).
- How to write a new test from scratch using the existing scaffolding.

Without this documentation, understanding the test suite requires manually
tracing through multiple files in `tests/common/` and `src/test.rs`.

---

## Changes

### Added: `docs/TESTING_WITH_MOCKS.md`

A 535-line developer guide structured as follows:

| Section | Contents |
|---|---|
| **Why Mock Contracts?** | Motivation — no live network in tests, what each mock replaces |
| **Project Test Layout** | Annotated directory tree of all test files and their roles |
| **MockWaveGuard** | Full annotated implementation, key design decisions |
| **MockToken** | Full annotated implementation, `mint` vs `transfer` distinction |
| **Unit Tests: TestEnv** | Inline setup pattern with numbered step-by-step commentary |
| **Integration Tests: TestContext** | Shared helper struct, method signatures explained |
| **Registering a mock contract** | `env.register()` usage |
| **mock_all_auths** | When to use it and when not to |
| **Generating test addresses** | `Address::generate` pattern |
| **Building repo hashes** | `BytesN::from_array` with different byte values |
| **Funding a pool** | Two-step mint + create pattern, why both steps are needed |
| **Advancing ledger time** | `env.ledger().set_timestamp()` for expiry tests |
| **Testing error paths** | `try_*` variants, result unwrapping pattern, table of meanings |
| **Writing a New Unit Test** | Complete copy-paste template |
| **Writing a New Integration Test** | Complete copy-paste template (happy + error path) |
| **TestContext API Reference** | Table of all fields and methods with types and descriptions |
| **Error Variants Reference** | Table of all 10 `Error` enum values with trigger conditions |

---

## What Was Reviewed

Before writing the guide, the following files were fully read to ensure the
documentation accurately reflects the actual implementation:

| File | Notes |
|---|---|
| `contracts/wave_milestone/src/test.rs` | Inline mocks, `TestEnv`, `setup()`, `fund_pool()`, all unit tests |
| `contracts/wave_milestone/tests/common/mod.rs` | `TestContext`, constants, helper methods |
| `contracts/wave_milestone/tests/common/mock_guard.rs` | `MockWaveGuard` implementation |
| `contracts/wave_milestone/tests/common/mock_token.rs` | `MockToken` implementation |
| `contracts/wave_milestone/tests/full_lifecycle.rs` | End-to-end integration test patterns |
| `contracts/wave_milestone/tests/duplicate_claim.rs` | Error path patterns with `try_*` |

All code samples in the guide are taken directly from the actual source and
integration test files to ensure accuracy.

---

## No Code Changes

This PR is documentation-only. No contract source, test logic, or CI
configuration was modified.

---

closes #86
