mod common;

use common::*;
use wave_milestone::types::Error;

#[test]
fn test_clawback_full_remaining_after_partial_claims() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;
    let bounty = DEFAULT_BOUNTY;

    ctx.fund_pool(pool_size);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &bounty);

    let balance_before = ctx.token_client().balance(&ctx.maintainer);
    ctx.advance_to_expiry();
    ctx.client().clawback_expired_funds(&ctx.maintainer);
    let balance_after = ctx.token_client().balance(&ctx.maintainer);

    let expected_return = pool_size - bounty;
    assert_eq!(balance_after - balance_before, expected_return);
    assert_eq!(ctx.client().milestone_balance(), 0);
}

#[test]
fn test_clawback_full_pool_no_claims() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;

    ctx.fund_pool(pool_size);

    let balance_before = ctx.token_client().balance(&ctx.maintainer);
    ctx.advance_to_expiry();
    ctx.client().clawback_expired_funds(&ctx.maintainer);
    let balance_after = ctx.token_client().balance(&ctx.maintainer);

    assert_eq!(balance_after - balance_before, pool_size);
}

#[test]
fn test_clawback_before_expiry_rejected() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    let result = ctx.client().try_clawback_expired_funds(&ctx.maintainer);

    assert_eq!(result.err().unwrap(), Ok(Error::PoolNotExpired));
}

#[test]
fn test_clawback_non_maintainer_rejected() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);
    ctx.advance_to_expiry();

    let result = ctx.client().try_clawback_expired_funds(&ctx.stranger);

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_clawback_when_pool_empty_rejected() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;

    ctx.fund_pool(pool_size);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &pool_size);

    ctx.advance_to_expiry();

    let result = ctx.client().try_clawback_expired_funds(&ctx.maintainer);

    assert_eq!(result.err().unwrap(), Ok(Error::NoFundsToClawback));
}

#[test]
fn test_clawback_pool_not_found() {
    let ctx = TestContext::new();
    ctx.advance_to_expiry();

    let result = ctx.client().try_clawback_expired_funds(&ctx.maintainer);

    assert_eq!(result.err().unwrap(), Ok(Error::PoolNotFound));
}

#[test]
fn test_release_bounty_after_clawback_rejected() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    ctx.advance_to_expiry();
    ctx.client().clawback_expired_funds(&ctx.maintainer);

    let result = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &1u32,
        &ctx.developer,
        &DEFAULT_BOUNTY,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::InsufficientPoolBalance));
}
