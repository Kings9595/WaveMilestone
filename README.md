# WaveMilestone

[![License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.81+-orange.svg)](https://www.rust-lang.org)
[![Soroban](https://img.shields.io/badge/Soroban-SDK%200.11+-blueviolet.svg)](https://soroban.stellar.org)
[![CI](https://github.com/Kings9595/WaveMilestone/actions/workflows/ci.yml/badge.svg)](https://github.com/Kings9595/WaveMilestone/actions/workflows/ci.yml)

**Automated Milestone & Issue Escrow Release** – WaveMilestone lets repository maintainers lock an asset budget against a GitHub Milestone. When issues tied to that milestone are closed and merged, the contract automatically releases micro-payouts to developers — no manual reconciliation, no end-of-month bottlenecks.

Built on Stellar Soroban and designed to integrate with [WaveGuard](https://github.com/anomalyco/waveguard) for access control, WaveMilestone keeps engineering momentum consistent during time-boxed sprints by automating financial closure alongside every PR merge.

---

## Table of Contents

- [Project Purpose](#project-purpose)
- [Why WaveMilestone?](#why-wavemilestone)
- [Target Users](#target-users)
- [Architecture Overview](#architecture-overview)
- [Asset Escrow Lifecycle](#asset-escrow-lifecycle)
- [Smart Contract Design](#smart-contract-design)
  - [Storage Strategy](#storage-strategy)
  - [Authentication Design](#authentication-design)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Build](#build)
  - [Test](#test)
- [Contract Methods](#contract-methods)
  - [`create_milestone_pool`](#create_milestone_pool)
  - [`release_issue_bounty`](#release_issue_bounty)
  - [`clawback_expired_funds`](#clawback_expired_funds)
  - [Helper / View Methods](#helper--view-methods)
- [Integration with WaveGuard](#integration-with-waveguard)
- [Security & Vulnerability Protections](#security--vulnerability-protections)
- [Testing](#testing)
- [Project Structure](#project-structure)
- [Contributing](#contributing)
- [License](#license)

---

## Project Purpose

WaveMilestone is an **automated milestone & issue escrow release** system designed to solve the fundamental problem of **predictable cash flow in time-boxed open-source development**. It bridges the gap between engineering velocity and financial settlement, ensuring that contributor compensation keeps pace with sprint cycles.

### Core Mission

In modern open-source projects, especially within the Stellar ecosystem, teams operate in intensive, time-boxed sprints (e.g., 1-week cycles). Traditional milestone payout systems create friction by:

1. **Decoupling financial settlement from engineering progress**
2. **Creating administrative bottlenecks** at arbitrary intervals
3. **Breaking development momentum** with payment delays

WaveMilestone addresses this by implementing a **trustless, on-chain escrow system** that automatically releases micro-payouts when engineering work is completed, eliminating manual reconciliation and end-of-month bottlenecks.

### Why This Matters in the Stellar Ecosystem

During intensive development sprints, predictable cash flow is as critical as predictable code flow. WaveMilestone ensures that **financial settlement keeps pace with engineering velocity**, removing the most common friction point in contributor retention and enabling sustainable open-source development.

### Target Impact

- **For Contributors**: Receive instant, on-chain payments the moment their PR is merged — no waiting for end-of-month processing.
- **For Maintainers**: Automate financial closure alongside every PR merge, reducing administrative overhead and maintaining sprint momentum.
- **For the Ecosystem**: Establish a reliable, trustless payment infrastructure that scales with open-source development velocity.

### Technical Philosophy

WaveMilestone is built on three core principles:

1. **Automation**: Eliminate manual processes through smart contract automation
2. **Trustlessness**: Use blockchain technology to remove counterparty risk
3. **Speed**: Enable micro-payouts that settle in near real-time

This foundation enables a new paradigm where financial settlement is an automatic byproduct of engineering progress, not a separate administrative task.

### Why It Matters in the Stellar Ecosystem

During intensive, time-boxed sprints (e.g., Drips Wave's 1-week cycles), predictable cash flow is as important as predictable code flow. WaveMilestone ensures that **financial settlement keeps pace with engineering velocity**, removing the most common friction point in contributor retention.

---

## Target Users

| Role | How They Use WaveMilestone |
|------|---------------------------|
| **Repository Maintainer** | Creates milestone pools, funds them with Stellar assets, and authorizes payouts as issues close. |
| **Contributor** | Receives instant, on-chain payments the moment their PR is merged — no waiting for end-of-month processing. |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                    WaveGuard                         │
│           (Access Registry / Maintainer ID)          │
└────────────┬─────────────────────────────┬──────────┘
             │                             │
             ▼                             ▼
┌──────────────────────────────────────────────────────┐
│                   WaveMilestone                      │
│              Escrow Asset Vault                       │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │ Milestone    │  │ Issue        │  │ Clawback   │ │
│  │ Pool Records │  │ Claim Logs   │  │ Mechanism  │ │
│  └──────────────┘  └──────────────┘  └────────────┘ │
└─────────────────────┬────────────────────────────────┘
                      │
                      ▼
┌──────────────────────────────────────────────────────┐
│            Stellar Asset Contract (SAC)               │
│              (Token Transfer Execution)                │
└──────────────────────────────────────────────────────┘
```

1. **Maintainer** calls `create_milestone_pool`, funding the contract with a SAC token budget.
2. **Maintainer** (or CI) calls `release_issue_bounty` with the issue ID, developer address, and amount. The contract verifies the maintainer's authority via WaveGuard.
3. **Contract** atomically marks the issue as claimed and transfers the payout.
4. **Maintainer** can claw back unclaimed funds after the milestone deadline via `clawback_expired_funds`.

---

## Asset Escrow Lifecycle

Tokens move through four distinct phases from deposit to final settlement:

1. **Pool Creation — Deposit & Lock**
   The maintainer calls `create_milestone_pool`, transferring `total_funds` from their wallet into the contract's asset vault. Tokens are locked on-chain and unavailable for withdrawal until the milestone concludes.

2. **Issue Payout — Transfer to Developer**
   When an issue is closed and merged, the maintainer (or CI) calls `release_issue_bounty`. The contract deducts the specified `amount` from the pool balance and atomically transfers it to the `developer` address. The issue is marked `completed = true`.

3. **Duplicate Claim Protection — No Double-Spend**
   Every claim is keyed by `(repo_hash, issue_id)`. If `release_issue_bounty` is called again for the same pair, the contract reverts with `BountyAlreadyClaimed` before touching any tokens — the pool balance is unchanged.

4. **Clawback — Unclaimed Funds Returned**
   After the milestone deadline, the maintainer calls `clawback_expired_funds`. Any remaining balance in the vault is transferred back to the maintainer, and the pool is cleared. No funds are ever stranded on-chain.

---

## Smart Contract Design

### Storage Strategy

| Storage Tier | Data | Key Schema | Rationale |
|-------------|------|-----------|-----------|
| **Instance** | Milestone pool metadata (asset address, total budget, guard contract ref) | `MilestonePoolKey` (singleton) | Persists for the contract's lifetime; cheap one-time bump. |
| **Instance** | Per-milestone aggregate data | `MilestoneDataKey(repo_hash)` | Tracks total allocated, remaining balance, and expiry per repo. |
| **Temporary** | Individual issue claim status | `IssueClaimKey(repo_hash, issue_id)` → `MilestoneAllocation` | Massive gas savings; claims are single-use and short-lived. |

### Authentication Design

Dual validation gates every payout:

1. **`maintainer.require_auth()`** – The calling address must be an authorized maintainer registered in WaveGuard.
2. **Recipient match** – The `developer` address in the call must match the contributor record associated with the issue.

Failed validation → `BountyAlreadyClaimed` or `UnauthorizedMaintainer` error (contract reverts).

### Data Types (`types.rs`)

```rust
#[derive(Clone)]
#[contracttype]
pub struct MilestoneAllocation {
    pub issue_id: u32,
    pub developer: Address,
    pub payment_amount: u128,
    pub completed: bool,
}
```

Allocation structs are stored under a composite key of `(repo_hash, issue_id)`, guaranteeing uniqueness across issues.

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.81+
- [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup)
- A Stellar testnet/futurenet identity for deployment

### Build

```bash
cd contracts/wave_milestone
cargo build --release
```

### Test

```bash
cargo test -- --nocapture
```

See [Testing](#testing) for details on the integration test suite.

---

## Contract Methods

### `create_milestone_pool`

Initializes a new milestone escrow pool. Transfers `total_funds` from the caller to the contract's asset vault.

```rust
fn create_milestone_pool(
    e: Env,
    guard_contract: Address,
    asset: Address,
    total_funds: u128,
) -> Result<(), Error>;
```

| Parameter | Description |
|-----------|-------------|
| `guard_contract` | Address of the deployed [WaveGuard](https://github.com/anomalyco/waveguard) contract instance. |
| `asset` | Stellar Asset Contract (SAC) token ID to use for payouts. |
| `total_funds` | Total budget to lock for this milestone (in the smallest unit of `asset`). |

**Auth:** `maintainer.require_auth()` (verified via WaveGuard).

---

### `release_issue_bounty`

Releases a micro-payout to a developer once their issue is closed and merged.

```rust
fn release_issue_bounty(
    e: Env,
    repo_hash: BytesN<32>,
    issue_id: u32,
    developer: Address,
    amount: u128,
) -> Result<(), Error>;
```

| Parameter | Description |
|-----------|-------------|
| `repo_hash` | SHA-256 hash identifying the GitHub repository (linked to a milestone). |
| `issue_id` | GitHub issue number (composite key with `repo_hash` prevents cross-repo collisions). |
| `developer` | Stellar address receiving the payout. |
| `amount` | Payout amount (must not exceed remaining pool balance). |

**Auth:** `maintainer.require_auth()` + recipient address validation.

**Safety:** Once `completed` is set to `true` for a given `(repo_hash, issue_id)` pair, any subsequent call reverts with `BountyAlreadyClaimed`.

---

### `clawback_expired_funds`

Returns unclaimed funds to the maintainer after a milestone deadline passes.

```rust
fn clawback_expired_funds(e: Env) -> Result<(), Error>;
```

**Auth:** `maintainer.require_auth()`. Only callable after milestone expiry.

---

### Helper / View Methods

*(Implement as needed for front-end integration)*

| Method | Returns | Description |
|--------|---------|-------------|
| `milestone_balance(e)` | `u128` | Current remaining balance in the milestone pool. |
| `is_claimed(repo_hash, issue_id)` | `bool` | Whether a specific issue has already been paid out. |
| `milestone_info(repo_hash)` | `MilestoneData` | Retrieve pool metadata for a given repo. |

---

## Integration with WaveGuard

WaveMilestone depends on [WaveGuard](https://github.com/anomalyco/waveguard) as its identity and access registry. During `create_milestone_pool` and `release_issue_bounty`, the contract calls into the WaveGuard instance to verify that the caller is an authorized maintainer.

The expected WaveGuard interface:

```rust
/// Returns true if `address` is an authorized maintainer.
fn is_maintainer(e: Env, address: Address) -> bool;
```

To integrate:

1. Deploy WaveGuard and register maintainer identities.
2. Pass the WaveGuard contract address as `guard_contract` when creating a milestone pool.
3. WaveMilestone handles the cross-contract calls internally.

---

## Security & Vulnerability Protections

### Duplicate Claim Prevention (Drain Attacks)

The contract uses a **strict composite storage key** combining `repo_hash + issue_id`. Once `completed` is set to `true`, any subsequent `release_issue_bounty` for that exact `(repo_hash, issue_id)` pair **immediately reverts** with a `BountyAlreadyClaimed` error code — before any token transfer occurs.

### Balance Overflow Protection

If a maintainer attempts to allocate more tokens than remain in the milestone pool, the contract detects the insufficient balance and gracefully reverts without locking up or losing any assets. Remaining funds stay accessible for future legitimate claims or clawback.

### Access Control

Only WaveGuard-verified maintainer addresses can create pools, release bounties, or claw back funds. Contributor addresses are **write-restricted** — they can only receive payouts, not trigger them.

### Testing Focus

The integration test suite covers:
- **Happy path:** Create pool, fund, release bounty, verify recipient balance.
- **Duplicate claim:** Attempt double-spend of the same issue ID → expect revert.
- **Over-allocation:** Attempt payout exceeding pool balance → expect graceful revert with no asset loss.
- **Clawback:** Claim expiry, return unclaimed funds to maintainer.
- **Unauthorized caller:** Non-maintainer addresses rejected.

---

## Testing

### Quick Start

```bash
# Run all tests (unit + integration) with output
cargo test -- --nocapture

# Run only unit tests
cargo test --package wave-milestone --lib -- --nocapture

# Run only integration tests
cargo test --package wave-milestone --test '*' -- --nocapture

# Run a specific test by name
cargo test test_duplicate_claim_rejected -- --nocapture
```

### Prerequisites for Integration Tests

Integration tests require the `wasm32-unknown-unknown` target:

```bash
rustup target add wasm32-unknown-unknown
```

### Running with Docker

```bash
# Run all tests in a Docker container
docker compose -f docker/docker-compose.yml run --rm test
```

### Test Suite Details

The test suite is organized into two categories:

**Unit tests** (`contracts/wave_milestone/src/test.rs`):
- Test individual function behavior and error paths
- Validate input validation and authentication logic
- Run quickly with no external dependencies

**Integration tests** (`contracts/wave_milestone/tests/*.rs`):
- Test cross-contract interactions with real Soroban environment
- Deploy a **mock SAC token** and **mock WaveGuard** contract
- Simulate the full milestone lifecycle

### Integration Test Scenarios

1. **Happy path** (`full_lifecycle.rs`): Create pool, fund, release bounty, verify recipient balance.
2. **Duplicate claim** (`duplicate_claim.rs`): Attempt double-spend of the same issue ID → expect revert.
3. **Over-allocation** (`over_allocation.rs`): Attempt payout exceeding pool balance → expect graceful revert with no asset loss.
4. **Clawback** (`clawback.rs`): Claim expiry, return unclaimed funds to maintainer.
5. **Unauthorized access** (`unauthorized_access.rs`): Non-maintainer addresses rejected.

### Using the Test Script

```bash
# Run all tests via the convenience script
./scripts/test.sh

# Run a specific test
./scripts/test.sh test_duplicate_claim_rejected

# Run with environment overrides
RUST_LOG=debug ./scripts/test.sh
```

---

## Project Structure

```
wave-milestone/
├── contracts/
│   └── wave_milestone/
│       ├── src/
│       │   ├── lib.rs          # Core ledger tracking and token distribution execution
│       │   └── types.rs        # Milestone allocations, error enums, and storage keys
│       └── Cargo.toml
├── tests/                       # Integration test suite
├── Cargo.toml                   # Workspace manifest
└── README.md
```

### Key Files

| File | Purpose |
|------|---------|
| `contracts/wave_milestone/src/lib.rs` | Core contract logic — pool creation, bounty release, clawback. |
| `contracts/wave_milestone/src/types.rs` | `MilestoneAllocation`, error enum (`BountyAlreadyClaimed`, `InsufficientPoolBalance`, `UnauthorizedMaintainer`), storage key definitions. |
| `contracts/wave_milestone/Cargo.toml` | Contract dependencies (soroban-sdk, waveguard-client, etc.). |
| `tests/*.rs` | Integration tests with mock SAC token and full lifecycle coverage. |

---

## Contributing

Contributions are welcome! Please follow the standard workflow:

1. Fork the repository.
2. Create a feature branch (`git checkout -b feat/my-feature`).
3. Commit your changes (`git commit -am 'Add feature'`).
4. Push to the branch (`git push origin feat/my-feature`).
5. Open a Pull Request.

Ensure all tests pass and new code includes appropriate test coverage. For major changes, please open an issue first to discuss what you would like to change.

---

## License

This project is licensed under the **Apache License 2.0**. See [LICENSE](LICENSE) for details.
