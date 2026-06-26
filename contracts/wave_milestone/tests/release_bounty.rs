mod common;

use common::*;
use wave_milestone::types::Error;

#[test]
fn test_release_bounty_success() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;
    let bounty = DEFAULT_BOUNTY;

    ctx.fund_pool(pool_size);

    let balance_before = ctx.token_client().balance(&ctx.developer);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &bounty);

    let balance_after = ctx.token_client().balance(&ctx.developer);
    assert_eq!(balance_after - balance_before, bounty);

    let remaining = ctx.client().milestone_balance();
    assert_eq!(remaining, pool_size - bounty);

    assert!(ctx.client().is_claimed(&ctx.repo_hash, &1u32));
}

#[test]
fn test_release_bounty_pool_not_found() {
    let ctx = TestContext::new();

    let result =
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    assert_eq!(result.err().unwrap(), Ok(Error::PoolNotFound));
}

#[test]
fn test_release_bounty_zero_amount() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    let result = ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &0u128);

    assert_eq!(result.err().unwrap(), Ok(Error::InvalidAmount));
}

#[test]
fn test_release_bounty_non_maintainer_rejected() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    let result =
        ctx.client().try_release_issue_bounty(&ctx.stranger, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_release_bounty_contract_address_as_developer_rejected() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    let result = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &1u32,
        &ctx.contract_id, // contract's own address — tokens would be locked
        &DEFAULT_BOUNTY,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::InvalidDeveloper));
}

#[test]
fn test_consecutive_bounties_different_issues() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;
    let bounty = DEFAULT_BOUNTY;

    ctx.fund_pool(pool_size);

    for issue_id in 1..=3u32 {
        ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &issue_id, &ctx.developer, &bounty);
    }

    let expected_remaining = pool_size - (bounty * 3);
    assert_eq!(ctx.client().milestone_balance(), expected_remaining);
}
