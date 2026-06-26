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

/// Verifies that `milestone_info` returns all pool fields correctly after creation.
#[test]
fn test_milestone_info_returns_correct_pool_data() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;

    ctx.token_client().mint(&ctx.maintainer, &pool_size);
    ctx.client().create_milestone_pool(&ctx.maintainer, &ctx.guard_id, &ctx.token_id, &pool_size, &ctx.expiry);

    let pool = ctx.client().milestone_info().expect("pool should exist");

    assert_eq!(pool.total_funds, pool_size);
    assert_eq!(pool.allocated_funds, 0);
    assert_eq!(pool.maintainer, ctx.maintainer);
    assert_eq!(pool.guard_contract, ctx.guard_id);
    assert_eq!(pool.asset, ctx.token_id);
    assert_eq!(pool.expiry, ctx.expiry);
}

#[test]
fn test_milestone_info_none_before_pool_creation() {
    let ctx = TestContext::new();
    assert!(ctx.client().milestone_info().is_none());
}

#[test]
fn test_create_pool_rejects_zero_amount() {
    let ctx = TestContext::new();

    let result =
        ctx.client().try_create_milestone_pool(&ctx.maintainer, &ctx.guard_id, &ctx.token_id, &0u128, &ctx.expiry);

    assert_eq!(result.err().unwrap(), Ok(Error::InvalidAmount));
}

/// Regression test: zero-fund pool creation must be a no-op.
///
/// Verifies that a rejected zero-amount call leaves no side effects:
/// - no pool is persisted in contract storage
/// - no tokens are transferred out of the maintainer's account
#[test]
fn test_create_pool_zero_funds_leaves_no_state() {
    let ctx = TestContext::new();
    let initial_balance = DEFAULT_POOL_FUNDS;
    ctx.token_client().mint(&ctx.maintainer, &initial_balance);

    let _ = ctx.client().try_create_milestone_pool(&ctx.maintainer, &ctx.guard_id, &ctx.token_id, &0u128, &ctx.expiry);

    // No pool should be stored.
    assert!(ctx.client().milestone_info().is_none());
    // No tokens should have moved.
    assert_eq!(ctx.token_client().balance(&ctx.maintainer), initial_balance);
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
fn test_create_pool_rejects_removed_maintainer() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;

    // Remove the maintainer from WaveGuard
    MockWaveGuardClient::new(&ctx.env, &ctx.guard_id).remove_maintainer(&ctx.maintainer);
    ctx.token_client().mint(&ctx.maintainer, &pool_size);

    let result =
        ctx.client().try_create_milestone_pool(&ctx.maintainer, &ctx.guard_id, &ctx.token_id, &pool_size, &ctx.expiry);

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
