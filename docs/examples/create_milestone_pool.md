# Example: `create_milestone_pool`

Initializes a milestone escrow pool by transferring `total_funds` of a Stellar Asset Contract (SAC) token from the maintainer into the contract vault. The pool remains active until `expiry`; after that, unclaimed funds can be reclaimed via `clawback_expired_funds`.

## Function Signature

```rust
pub fn create_milestone_pool(
    env: Env,
    maintainer: Address,
    guard_contract: Address,
    asset: Address,
    total_funds: u128,
    expiry: u64,
) -> Result<(), Error>;
```

## Prerequisites

1. **Deploy WaveGuard** and register the maintainer address:
   ```bash
   stellar contract invoke --id $WAVEGUARD_ID -- add_maintainer --address $MAINTAINER_ADDRESS
   ```

2. **Have a SAC token** (`$TOKEN_ID`) with sufficient balance in the maintainer's account.

3. **Authorize the contract** to pull `total_funds` from the maintainer (handled automatically by `require_auth`; the signer must approve the transaction).

## Rust Test Example

```rust
mod common;
use common::*;

#[test]
fn test_create_milestone_pool() {
    let ctx = TestContext::new();
    let pool_funds = 10_000_000_000u128; // 10_000 tokens (7 decimals)

    // Fund the maintainer's token balance
    ctx.token_client().mint(&ctx.maintainer, &pool_funds);

    // Create the pool — transfers funds into the contract vault
    ctx.client().create_milestone_pool(
        &ctx.maintainer,
        &ctx.guard_id,
        &ctx.token_id,
        &pool_funds,
        &ctx.expiry,  // Unix timestamp (seconds); must be in the future
    );

    // Verify pool state
    let pool = ctx.client().milestone_info().unwrap();
    assert_eq!(pool.total_funds, pool_funds);
    assert_eq!(pool.allocated_funds, 0);
    assert_eq!(pool.maintainer, ctx.maintainer);
    assert_eq!(ctx.client().milestone_balance(), pool_funds);
}
```

## CLI Invocation

```bash
stellar contract invoke \
  --id $CONTRACT_ID \
  --source $MAINTAINER_SECRET_KEY \
  --network testnet \
  -- create_milestone_pool \
  --maintainer $MAINTAINER_ADDRESS \
  --guard_contract $WAVEGUARD_ID \
  --asset $TOKEN_ID \
  --total_funds 10000000000 \
  --expiry 1782000000
```

> `expiry` is a Unix timestamp in seconds. Use `date -d "+30 days" +%s` to compute a value 30 days from now.

## Common Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `InvalidAmount` (9) | `total_funds` is `0` | Pass a non-zero budget. |
| `UnauthorizedMaintainer` (5) | Caller is not registered in WaveGuard | Register the address with WaveGuard before calling. |
| `ExpiryInPast` (10) | `expiry` ≤ current ledger timestamp | Use a future Unix timestamp. |
