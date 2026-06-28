mod common;

use common::*;

/// Test that milestone balance correctly reflects the remaining funds after a bounty release.
#[test]
fn test_milestone_balance_after_release() {
    let ctx = TestContext::new();
    let pool_size = 100_000_000_000u128;
    let bounty = 25_000_000_000u128;

    // Create and fund the pool
    ctx.fund_pool(pool_size);
    assert_eq!(ctx.client().milestone_balance(), pool_size);

    // Release a bounty
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &bounty);

    // Check balance is decreased by exactly the bounty amount
    let expected_remaining = pool_size - bounty;
    assert_eq!(ctx.client().milestone_balance(), expected_remaining);
}

/// Test that multiple bounty releases correctly decrement the milestone balance.
#[test]
fn test_milestone_balance_after_multiple_releases() {
    let ctx = TestContext::new();
    let pool_size = 100_000_000_000u128;
    let bounties = vec![10_000_000_000u128, 20_000_000_000u128, 15_000_000_000u128];

    ctx.fund_pool(pool_size);
    assert_eq!(ctx.client().milestone_balance(), pool_size);

    let mut total_released = 0u128;
    for (idx, &bounty) in bounties.iter().enumerate() {
        let issue_id = (idx + 1) as u32;
        ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &issue_id, &ctx.developer, &bounty);
        total_released += bounty;

        let expected_balance = pool_size - total_released;
        assert_eq!(ctx.client().milestone_balance(), expected_balance);
    }
}

/// Test that milestone balance is consistent with milestone_info().
#[test]
fn test_milestone_balance_consistency_with_info() {
    let ctx = TestContext::new();
    let pool_size = 50_000_000_000u128;
    let bounty = 10_000_000_000u128;

    ctx.fund_pool(pool_size);

    // Before release
    let info_before = ctx.client().milestone_info().unwrap();
    assert_eq!(ctx.client().milestone_balance(), info_before.remaining_balance());

    // After release
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &bounty);
    let info_after = ctx.client().milestone_info().unwrap();
    assert_eq!(ctx.client().milestone_balance(), info_after.remaining_balance());

    // Verify arithmetic
    assert_eq!(info_after.allocated_funds, info_before.allocated_funds + bounty);
    assert_eq!(info_after.total_funds, info_before.total_funds);
}
