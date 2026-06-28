# Testing with Mock Contracts

This guide explains how WaveMilestone's test suite uses mock contracts to test
cross-contract interactions without deploying real external dependencies. It
covers the mock implementations, the test helpers, and the patterns used across
both unit and integration tests.

---

## Table of Contents

- [Why Mock Contracts?](#why-mock-contracts)
- [Project Test Layout](#project-test-layout)
- [The Mock Contracts](#the-mock-contracts)
  - [MockWaveGuard](#mockwaveguard)
  - [MockToken](#mocktoken)
- [Test Environment Setup](#test-environment-setup)
  - [Unit Tests: inline TestEnv](#unit-tests-inline-testenv)
  - [Integration Tests: shared TestContext](#integration-tests-shared-testcontext)
- [Core Testing Patterns](#core-testing-patterns)
  - [Registering a mock contract](#registering-a-mock-contract)
  - [Bypassing auth with mock_all_auths](#bypassing-auth-with-mock_all_auths)
  - [Generating test addresses](#generating-test-addresses)
  - [Building repo hashes](#building-repo-hashes)
  - [Funding a pool](#funding-a-pool)
  - [Advancing ledger time](#advancing-ledger-time)
  - [Testing error paths with try_*](#testing-error-paths-with-try_)
- [Writing a New Test](#writing-a-new-test)
  - [Unit test (src/test.rs)](#unit-test-srctestrs)
  - [Integration test (tests/)](#integration-test-tests)
- [Reference: TestContext API](#reference-testcontext-api)
- [Reference: Error Variants](#reference-error-variants)

---

## Why Mock Contracts?

WaveMilestone calls two external contracts at runtime:

| Dependency | Purpose |
|---|---|
| **WaveGuard** | Authoritative registry — answers `is_maintainer(address)` |
| **Token (SAC)** | Stellar Asset Contract — executes `transfer` and `balance` |

In the Soroban test environment there are no live network contracts. Mock
contracts are lightweight, in-process Soroban contracts that implement the same
interface as the real dependency but store state purely in the test environment.
This lets tests:

- Control exactly which addresses are maintainers.
- Mint arbitrary token balances without going through the full SAC machinery.
- Assert on contract state after each call without needing an RPC node.

---

## Project Test Layout

```
contracts/wave_milestone/
├── src/
│   ├── lib.rs           # Contract implementation
│   ├── types.rs         # Data types, errors, storage keys
│   └── test.rs          # Unit tests with inline mock contracts
└── tests/
    ├── common/
    │   ├── mod.rs        # TestContext struct and shared helpers
    │   ├── mock_guard.rs # MockWaveGuard contract
    │   └── mock_token.rs # MockToken contract
    ├── full_lifecycle.rs
    ├── duplicate_claim.rs
    ├── over_allocation.rs
    ├── clawback.rs
    ├── unauthorized_access.rs
    ├── release_bounty.rs
    ├── create_pool.rs
    └── claim_manipulation.rs
```

Unit tests live in `src/test.rs` and define their own inline mocks (same
logic, slightly different struct names). Integration tests in `tests/` share
the mocks and helpers defined in `tests/common/`.

---

## The Mock Contracts

### MockWaveGuard

**File:** `tests/common/mock_guard.rs`

Implements the WaveGuard `is_maintainer` interface. Maintainer status is stored
as a `bool` under a per-address key in Instance storage.

```rust
#[contracttype]
#[derive(Clone)]
pub enum MockGuardKey {
    Maintainer(Address),
}

#[contract]
pub struct MockWaveGuard;

#[contractimpl]
impl MockWaveGuard {
    /// Returns true if `address` has been added as a maintainer.
    pub fn is_maintainer(env: Env, address: Address) -> bool {
        env.storage()
            .instance()
            .get::<_, bool>(&MockGuardKey::Maintainer(address))
            .unwrap_or(false)
    }

    /// Grants maintainer status to `address`.
    pub fn add_maintainer(env: Env, address: Address) {
        env.storage()
            .instance()
            .set(&MockGuardKey::Maintainer(address), &true);
    }

    /// Revokes maintainer status from `address`.
    pub fn remove_maintainer(env: Env, address: Address) {
        env.storage()
            .instance()
            .set(&MockGuardKey::Maintainer(address), &false);
    }
}
```

Key design decisions:
- `unwrap_or(false)` — any address not explicitly added is not a maintainer.
- `remove_maintainer` sets the key to `false` rather than deleting it, which
  models the real WaveGuard revocation behavior.

### MockToken

**File:** `tests/common/mock_token.rs`

Implements the SAC token interface (`transfer`, `balance`) plus a `mint` helper
that does not exist on real SAC tokens but is needed to bootstrap test balances.

```rust
#[contracttype]
#[derive(Clone)]
pub enum MockTokenKey {
    Balance(Address),
    Admin,
}

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    /// Call once after registering to set the admin address.
    pub fn init(env: Env, admin: Address) {
        env.storage().instance().set(&MockTokenKey::Admin, &admin);
    }

    /// Mint `amount` tokens directly into `to`'s balance. Test-only.
    pub fn mint(env: Env, to: Address, amount: u128) {
        let bal = env.storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&MockTokenKey::Balance(to), &(bal + amount));
    }

    /// Transfer `amount` from `from` to `to`. Calls from.require_auth().
    pub fn transfer(env: Env, from: Address, to: Address, amount: u128) {
        from.require_auth();
        let from_bal = env.storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(from.clone()))
            .unwrap_or(0);
        assert!(from_bal >= amount, "insufficient balance");
        let to_bal = env.storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&MockTokenKey::Balance(from), &(from_bal - amount));
        env.storage()
            .instance()
            .set(&MockTokenKey::Balance(to), &(to_bal + amount));
    }

    /// Returns the token balance for `id`.
    pub fn balance(env: Env, id: Address) -> u128 {
        env.storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(id))
            .unwrap_or(0)
    }
}
```

Key design decisions:
- `transfer` calls `from.require_auth()` to mirror the real SAC, so tests that
  do not call `env.mock_all_auths()` will fail on unauthorized transfers.
- `mint` deliberately has no auth guard — it is a test-only escape hatch to
  seed balances without going through a real issuance flow.
- Balances default to `0` via `unwrap_or(0)`, so addresses that have never
  received tokens don't need to be initialized.

---

## Test Environment Setup

### Unit Tests: inline TestEnv

Unit tests in `src/test.rs` define their own `TestEnv` struct and `setup()`
function that mirrors the integration `TestContext` but lives inline in the
same file (keeping unit tests self-contained).

```rust
struct TestEnv {
    env: Env,
    maintainer: Address,
    developer: Address,
    stranger: Address,
    contract_id: Address,
    guard_id: Address,
    token_id: Address,
    repo_hash: BytesN<32>,
    expiry: u64,
}

fn setup() -> TestEnv {
    let env = Env::default();
    env.mock_all_auths();                           // 1. bypass all auth checks

    let maintainer = Address::generate(&env);       // 2. generate unique addresses
    let developer  = Address::generate(&env);
    let stranger   = Address::generate(&env);

    let guard_id = env.register(MockWaveGuard, ()); // 3. deploy mock contracts
    MockWaveGuardClient::new(&env, &guard_id)
        .add_maintainer(&maintainer);               // 4. configure mock state

    let token_id = env.register(MockToken, ());
    MockTokenClient::new(&env, &token_id)
        .init(&maintainer);

    let contract_id = env.register(WaveMilestoneContract, ()); // 5. deploy subject

    let repo_hash = BytesN::from_array(&env, &[0u8; 32]);
    let expiry = env.ledger().timestamp() + 2_592_000; // 30 days

    TestEnv { env, maintainer, developer, stranger,
              contract_id, guard_id, token_id, repo_hash, expiry }
}
```

### Integration Tests: shared TestContext

Integration tests use `TestContext` from `tests/common/mod.rs`. It has the same
setup logic but exposes helper methods to reduce boilerplate across test files.

```rust
pub struct TestContext {
    pub env: Env,
    pub maintainer: Address,
    pub developer: Address,
    pub developer_two: Address,
    pub stranger: Address,
    pub contract_id: Address,
    pub guard_id: Address,
    pub token_id: Address,
    pub repo_hash: BytesN<32>,
    pub repo_hash_two: BytesN<32>,
    pub expiry: u64,
}

impl TestContext {
    pub fn new() -> Self { /* same setup steps as above */ }

    /// Mint `amount` to maintainer and create the pool.
    pub fn fund_pool(&self, amount: u128) { ... }

    /// Returns a client bound to the WaveMilestone contract.
    pub fn client(&self) -> WaveMilestoneContractClient<'_> { ... }

    /// Returns a client bound to the mock token.
    pub fn token_client(&self) -> MockTokenClient<'_> { ... }

    /// Advances ledger time past the pool expiry.
    pub fn advance_to_expiry(&self) { ... }
}
```

Every integration test file begins with:

```rust
mod common;
use common::*;
```

---

## Core Testing Patterns

### Registering a mock contract

Use `env.register(ContractStruct, ())` to deploy a contract into the in-process
Soroban environment. It returns the contract's `Address`.

```rust
let guard_id = env.register(MockWaveGuard, ());
let token_id = env.register(MockToken, ());
let contract_id = env.register(WaveMilestoneContract, ());
```

The second argument is the constructor argument tuple; `()` means no constructor.

### Bypassing auth with mock_all_auths

Most tests call `env.mock_all_auths()` immediately after creating the
environment. This makes all `require_auth()` checks succeed automatically,
so tests focus on business logic rather than signature construction.

```rust
let env = Env::default();
env.mock_all_auths();
```

If you want to test that a specific auth check fires, do **not** call
`mock_all_auths()` and instead set up the auth manually — or simply observe
that the `try_*` call returns `Err` as the test contract's internal `assert!`
will panic.

### Generating test addresses

`Address::generate(&env)` produces a random, unique `Address` inside the test
environment. Use it instead of hardcoding addresses.

```rust
let maintainer = Address::generate(&env);
let developer  = Address::generate(&env);
```

### Building repo hashes

`BytesN::from_array(&env, &[byte; 32])` creates a 32-byte hash. Use different
byte values to represent different repositories.

```rust
let repo_a = BytesN::from_array(&env, &[0u8; 32]); // all zeros → repo A
let repo_b = BytesN::from_array(&env, &[1u8; 32]); // all ones  → repo B
```

### Funding a pool

Minting tokens and creating the pool is a two-step sequence because the contract
pulls funds from the caller during `create_milestone_pool`.

```rust
// Mint tokens into maintainer's mock balance
MockTokenClient::new(&env, &token_id).mint(&maintainer, &pool_size);

// Create pool — contract calls token.transfer internally
client.create_milestone_pool(
    &maintainer,
    &guard_id,
    &token_id,
    &pool_size,
    &expiry,
);
```

In integration tests the `ctx.fund_pool(amount)` helper wraps this pattern.

### Advancing ledger time

`clawback_expired_funds` requires `now >= pool.expiry`. Advance the simulated
ledger clock with `env.ledger().set_timestamp(unix_seconds)`.

```rust
// Move past the expiry stored in the pool
env.ledger().set_timestamp(expiry + 1);

client.clawback_expired_funds(&maintainer);
```

In integration tests use `ctx.advance_to_expiry()`.

### Testing error paths with try_*

Every contract method has a `try_` variant generated by `soroban-sdk` that
returns `Result<T, Result<Error, InvokeError>>` instead of panicking. Use it
for any test that expects a failure.

```rust
let result = client.try_release_issue_bounty(
    &maintainer,
    &repo_hash,
    &1u32,
    &developer,
    &bounty,
);

// The outer Ok/Err is the invocation result;
// the inner Ok(Error) is a contract-returned error code.
assert_eq!(result.err().unwrap(), Ok(Error::BountyAlreadyClaimed));
```

Pattern breakdown:

| Expression | Meaning |
|---|---|
| `result.is_ok()` | Call succeeded, no error |
| `result.err().unwrap()` | Call failed — extract the error wrapper |
| `Ok(Error::VariantName)` | Contract returned this specific error variant |

---

## Writing a New Test

### Unit test (src/test.rs)

```rust
#[test]
fn test_my_scenario() {
    let t = setup();                          // initialize env + mocks

    // Arrange: configure state
    MockTokenClient::new(&t.env, &t.token_id).mint(&t.maintainer, &5_000_000_000);

    // Act: call the contract under test
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).create_milestone_pool(
        &t.maintainer,
        &t.guard_id,
        &t.token_id,
        &5_000_000_000,
        &t.expiry,
    );

    // Assert: verify expected state
    let balance = WaveMilestoneContractClient::new(&t.env, &t.contract_id)
        .milestone_balance();
    assert_eq!(balance, 5_000_000_000);
}
```

### Integration test (tests/)

Create a new file, e.g. `tests/my_scenario.rs`:

```rust
mod common;
use common::*;
use wave_milestone::types::Error; // only needed if asserting on error variants

#[test]
fn test_my_scenario() {
    let ctx = TestContext::new();

    // Arrange
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    // Act
    ctx.client().release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &1u32,
        &ctx.developer,
        &DEFAULT_BOUNTY,
    );

    // Assert
    let balance_after = ctx.token_client().balance(&ctx.developer);
    assert_eq!(balance_after, DEFAULT_BOUNTY);
}

#[test]
fn test_my_error_scenario() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    let result = ctx.client().try_release_issue_bounty(
        &ctx.stranger,         // not a maintainer
        &ctx.repo_hash,
        &1u32,
        &ctx.developer,
        &DEFAULT_BOUNTY,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}
```

---

## Reference: TestContext API

| Method / Field | Type | Description |
|---|---|---|
| `TestContext::new()` | `-> Self` | Create a fresh env with all mocks registered and maintainer authorized |
| `ctx.fund_pool(amount)` | `u128 -> ()` | Mint `amount` to maintainer and create the pool |
| `ctx.client()` | `-> WaveMilestoneContractClient` | Client bound to the contract under test |
| `ctx.token_client()` | `-> MockTokenClient` | Client bound to the mock token |
| `ctx.advance_to_expiry()` | `-> ()` | Set ledger timestamp to `expiry + 1` |
| `ctx.env` | `Env` | The Soroban test environment |
| `ctx.maintainer` | `Address` | Address with WaveGuard maintainer status |
| `ctx.developer` | `Address` | Primary developer address (recipient) |
| `ctx.developer_two` | `Address` | Secondary developer address |
| `ctx.stranger` | `Address` | Address with no special permissions |
| `ctx.guard_id` | `Address` | Deployed MockWaveGuard contract address |
| `ctx.token_id` | `Address` | Deployed MockToken contract address |
| `ctx.contract_id` | `Address` | Deployed WaveMilestoneContract address |
| `ctx.repo_hash` | `BytesN<32>` | `[0u8; 32]` — primary test repo hash |
| `ctx.repo_hash_two` | `BytesN<32>` | `[1u8; 32]` — secondary test repo hash |
| `ctx.expiry` | `u64` | `ledger.timestamp() + 2_592_000` (30 days) |
| `DEFAULT_POOL_FUNDS` | `u128` | `10_000_000_000` |
| `DEFAULT_BOUNTY` | `u128` | `2_500_000_000` |

---

## Reference: Error Variants

| Variant | Value | Triggered when |
|---|---|---|
| `PoolNotFound` | 1 | `release_issue_bounty` or `clawback` called before `create_milestone_pool` |
| `PoolNotExpired` | 2 | `clawback_expired_funds` called before `pool.expiry` |
| `BountyAlreadyClaimed` | 3 | Same `(repo_hash, issue_id)` pair claimed twice |
| `InsufficientPoolBalance` | 4 | `amount > pool.remaining_balance()` |
| `UnauthorizedMaintainer` | 5 | Caller not registered in WaveGuard |
| `UnauthorizedCaller` | 6 | `clawback` caller does not match `pool.maintainer` |
| `NoFundsToClawback` | 7 | `remaining_balance()` is zero at clawback time |
| `TransferFailed` | 8 | Reserved for token transfer failures |
| `InvalidAmount` | 9 | `amount == 0` passed to pool creation or bounty release |
| `ExpiryInPast` | 10 | `expiry <= env.ledger().timestamp()` at pool creation |
