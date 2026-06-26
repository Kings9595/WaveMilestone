# Add Dedicated CI Check for Cargo Clippy Warnings

## Summary

This PR adds a dedicated GitHub Actions workflow for `cargo clippy` linting, ensuring that all Rust code in the WaveMilestone workspace is checked for warnings on every push and pull request. Any clippy warning is treated as a hard CI failure (`-D warnings`), preventing warning-laden code from being merged.

---

## Problem

While the existing `ci.yml` already contains a `Clippy` step inside the `lint` job (which bundles formatting checks alongside linting), there was no dedicated, standalone workflow specifically for `cargo clippy`. A dedicated workflow:

- Makes it immediately clear at a glance in the Actions tab whether linting is passing or failing.
- Allows the clippy check to be referenced independently in branch protection rules.
- Provides a clean, focused entry point for future clippy configuration changes (e.g., adding `--features`, per-crate flags, or nightly lints) without touching the broader CI pipeline.

---

## Changes

### Added: `.github/workflows/clippy.yml`

A new GitHub Actions workflow file that:

| Property | Value |
|---|---|
| **Trigger** | `push` to `main` / `develop`, `pull_request` targeting `main` |
| **Runner** | `ubuntu-latest` |
| **Toolchain** | `dtolnay/rust-toolchain@stable` with the `clippy` component |
| **Targets** | `wasm32-unknown-unknown`, `wasm32v1-none` (matches project toolchain) |
| **Cache** | `Swatinem/rust-cache@v2` scoped to `contracts/wave_milestone` |
| **Command** | `cargo clippy --workspace --all-targets -- -D warnings` |

The `-D warnings` flag promotes all clippy warnings to hard errors, meaning any lint issue causes the workflow to fail and block the PR.

---

## Source Code Audit

Before adding the CI check, all source files were reviewed for pre-existing clippy warnings:

| File | Status |
|---|---|
| `contracts/wave_milestone/src/lib.rs` | ✅ Clean |
| `contracts/wave_milestone/src/types.rs` | ✅ Clean |
| `contracts/wave_milestone/src/events.rs` | ✅ Clean |
| `contracts/wave_milestone/src/test.rs` | ✅ Clean |
| Integration tests (`tests/*.rs`) | ✅ Clean |

No pre-existing warnings were found. The code already follows clean Rust idioms:
- Uses `saturating_sub` for arithmetic safety.
- Uses `is_some_and` instead of `map(|x| x.condition).unwrap_or(false)`.
- No unused imports, dead code, or shadowed variables.
- `#[must_use]` applied to `remaining_balance()` as appropriate.

---

## Existing `ci.yml` Relationship

The `lint` job in `ci.yml` retains its existing `Clippy` step — this PR does **not** remove it. Both checks serve a purpose:

- **`ci.yml` lint job**: Bundles `rustfmt` + `clippy` in a single gated job that other CI jobs (`contract-test`, `test`, `build`) depend on via `needs: lint`.
- **`clippy.yml`** (new): Standalone, dedicated clippy check — runs independently, visible as its own status check, and can be referenced directly in branch protection rules.

---

## Testing

The workflow was validated by:

1. Reviewing all contract source files for existing clippy lint issues (none found).
2. Verifying the workflow YAML is syntactically correct and uses the same toolchain targets defined in `rust-toolchain.toml`.
3. Confirming `clippy.toml` settings (`allow-expect-in-tests`, `allow-unwrap-in-tests`, `msrv = "1.81.0"`) are respected by the `cargo clippy` invocation (clippy reads `clippy.toml` automatically).

---

## How to Verify

After merging, confirm:

```bash
# The workflow appears in the Actions tab under "Clippy"
# Any PR that introduces a clippy warning will show a failing check

# To test locally:
cargo clippy --workspace --all-targets -- -D warnings
```

---

closes #91
