mod common;

use common::*;
use wave_milestone::types::Error;

#[test]
fn test_create_milestone_pool_success() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;

    ctx.token_client().mint(&ctx.maintainer, &pool_size);
    ctx.client().create_milestone_pool(
        &ctx.maintainer,
        &ctx.guard_id,
        &ctx.token_id,
        &pool_size,
        &ctx.expiry,
    );

    let balance = ctx.client().milestone_balance();
    assert_eq!(balance, pool_size);

    let info = ctx.client().milestone_info();
    assert!(info.is_some());
    let pool = info.unwrap();
    assert_eq!(pool.total_funds, pool_size);
    assert_eq!(pool.allocated_funds, 0);
    assert_eq!(pool.maintainer, ctx.maintainer);
}

#[test]
fn test_create_pool_rejects_zero_amount() {
    let ctx = TestContext::new();

    let result =
        ctx.client().try_create_milestone_pool(&ctx.maintainer, &ctx.guard_id, &ctx.token_id, &0u128, &ctx.expiry);

    assert_eq!(result.err().unwrap(), Ok(Error::InvalidAmount));
}

#[test]
fn test_create_pool_rejects_unauthorized() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;

    ctx.token_client().mint(&ctx.stranger, &pool_size);

    let result =
        ctx.client().try_create_milestone_pool(&ctx.stranger, &ctx.guard_id, &ctx.token_id, &pool_size, &ctx.expiry);

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_create_pool_rejects_past_expiry() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;
    let past_expiry = ctx.env.ledger().timestamp();

    ctx.token_client().mint(&ctx.maintainer, &pool_size);

    let result =
        ctx.client().try_create_milestone_pool(&ctx.maintainer, &ctx.guard_id, &ctx.token_id, &pool_size, &past_expiry);

    assert_eq!(result.err().unwrap(), Ok(Error::ExpiryInPast));
}
