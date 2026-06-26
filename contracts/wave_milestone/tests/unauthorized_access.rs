mod common;

use common::*;
use wave_milestone::types::Error;

#[test]
fn test_unregistered_maintainer_cannot_create_pool() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;

    ctx.token_client().mint(&ctx.stranger, &pool_size);

    let result =
        ctx.client().try_create_milestone_pool(&ctx.stranger, &ctx.guard_id, &ctx.token_id, &pool_size, &ctx.expiry);

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_unregistered_maintainer_cannot_release_bounty() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    let result =
        ctx.client().try_release_issue_bounty(&ctx.stranger, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_stranger_cannot_clawback() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);
    ctx.advance_to_expiry();

    let result = ctx.client().try_clawback_expired_funds(&ctx.stranger);

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_removed_maintainer_loses_access() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    // Remove maintainer from WaveGuard
    MockWaveGuardClient::new(&ctx.env, &ctx.guard_id).remove_maintainer(&ctx.maintainer);

    let result =
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_unregistered_maintainer_bounty_released_after_removal() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    // Remove then try
    MockWaveGuardClient::new(&ctx.env, &ctx.guard_id).remove_maintainer(&ctx.maintainer);

    let result =
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}
