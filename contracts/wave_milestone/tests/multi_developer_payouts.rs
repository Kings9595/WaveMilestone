mod common;

use common::*;

/// Two developers receive different bounty amounts from the same pool.
/// Verifies each developer's final balance matches their individual payout.
#[test]
fn test_two_developers_receive_different_payouts() {
    let ctx = TestContext::new();
    let bounty_one = 3_000_000_000u128;
    let bounty_two = 1_500_000_000u128;

    ctx.fund_pool(bounty_one + bounty_two);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &bounty_one);
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &2u32, &ctx.developer_two, &bounty_two);

    assert_eq!(ctx.token_client().balance(&ctx.developer), bounty_one);
    assert_eq!(ctx.token_client().balance(&ctx.developer_two), bounty_two);
    assert_eq!(ctx.client().milestone_balance(), 0);
}

/// Multiple developers across different repos each receive the correct amount.
#[test]
fn test_developers_across_repos_receive_correct_amounts() {
    let ctx = TestContext::new();
    let bounty_one = 4_000_000_000u128;
    let bounty_two = 2_000_000_000u128;

    ctx.fund_pool(bounty_one + bounty_two);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &bounty_one);
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash_two, &1u32, &ctx.developer_two, &bounty_two);

    assert_eq!(ctx.token_client().balance(&ctx.developer), bounty_one);
    assert_eq!(ctx.token_client().balance(&ctx.developer_two), bounty_two);
}

/// The pool balance decreases correctly as multiple developers are paid out.
#[test]
fn test_pool_balance_decreases_with_each_developer_payout() {
    let ctx = TestContext::new();
    let pool_size = 10_000_000_000u128;
    let payouts = [(1u32, 1_000_000_000u128), (2u32, 2_000_000_000u128), (3u32, 3_000_000_000u128)];

    ctx.fund_pool(pool_size);

    let mut expected_remaining = pool_size;
    for (issue_id, amount) in &payouts {
        ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, issue_id, &ctx.developer_two, amount);
        expected_remaining -= amount;
        assert_eq!(ctx.client().milestone_balance(), expected_remaining);
    }
}
