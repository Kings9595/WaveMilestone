mod common;

use common::*;
use wave_milestone::types::Error;

/// End-to-end: multiple developers receive bounties from the same pool.
/// Verifies per-developer balances, correct pool depletion, and that the
/// clawback returns only what was not yet allocated.
#[test]
fn test_full_lifecycle_multi_developer() {
    let ctx = TestContext::new();
    let pool_size = 60_000_000_000u128;
    let bounty_one = 20_000_000_000u128;
    let bounty_two = 15_000_000_000u128;

    ctx.fund_pool(pool_size);

    // Two different developers each receive a bounty for different issues.
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &bounty_one);
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &2u32, &ctx.developer_two, &bounty_two);

    assert_eq!(ctx.token_client().balance(&ctx.developer), bounty_one);
    assert_eq!(ctx.token_client().balance(&ctx.developer_two), bounty_two);
    assert_eq!(ctx.client().milestone_balance(), pool_size - bounty_one - bounty_two);

    // Clawback returns only the unallocated remainder to the maintainer.
    let before = ctx.token_client().balance(&ctx.maintainer);
    ctx.advance_to_expiry();
    ctx.client().clawback_expired_funds(&ctx.maintainer);
    let after = ctx.token_client().balance(&ctx.maintainer);

    assert_eq!(after - before, pool_size - bounty_one - bounty_two);
    assert_eq!(ctx.client().milestone_balance(), 0);
}

/// End-to-end: `milestone_info` tracks `allocated_funds` accurately at
/// every phase of the lifecycle (creation → claim → claim → clawback).
#[test]
fn test_full_lifecycle_pool_accounting_tracks_allocated_funds() {
    let ctx = TestContext::new();
    let pool_size = 90_000_000_000u128;
    let first = 30_000_000_000u128;
    let second = 25_000_000_000u128;

    ctx.fund_pool(pool_size);

    // After creation: nothing allocated.
    let pool = ctx.client().milestone_info().unwrap();
    assert_eq!(pool.total_funds, pool_size);
    assert_eq!(pool.allocated_funds, 0);

    // After first bounty.
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &10u32, &ctx.developer, &first);
    let pool = ctx.client().milestone_info().unwrap();
    assert_eq!(pool.allocated_funds, first);
    assert_eq!(pool.total_funds, pool_size);

    // After second bounty.
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &11u32, &ctx.developer, &second);
    let pool = ctx.client().milestone_info().unwrap();
    assert_eq!(pool.allocated_funds, first + second);
    assert_eq!(pool.total_funds, pool_size);

    // Clawback: total_funds collapses to allocated_funds, remainder returns to maintainer.
    ctx.advance_to_expiry();
    ctx.client().clawback_expired_funds(&ctx.maintainer);
    let pool = ctx.client().milestone_info().unwrap();
    assert_eq!(pool.total_funds, pool.allocated_funds, "after clawback total_funds must equal allocated_funds");
    assert_eq!(ctx.client().milestone_balance(), 0);
}

/// End-to-end: bounties claimed across two different repo hashes from
/// the same pool.  Verifies that claim guards are scoped per repo and
/// that the remaining balance accounts for all payouts combined.
#[test]
fn test_full_lifecycle_multi_repo() {
    let ctx = TestContext::new();
    let pool_size = 80_000_000_000u128;
    let bounty_a = 18_000_000_000u128;
    let bounty_b = 22_000_000_000u128;

    ctx.fund_pool(pool_size);

    // Issue #5 in repo_hash (repo A).
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &5u32, &ctx.developer, &bounty_a);
    // Issue #5 in repo_hash_two (repo B) — same issue number, different repo.
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash_two, &5u32, &ctx.developer_two, &bounty_b);

    // Each claim is independently recorded.
    assert!(ctx.client().is_claimed(&ctx.repo_hash, &5u32));
    assert!(ctx.client().is_claimed(&ctx.repo_hash_two, &5u32));

    // Re-claiming either is rejected.
    assert_eq!(
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &5u32, &ctx.developer, &bounty_a)
            .err().unwrap(),
        Ok(Error::BountyAlreadyClaimed)
    );
    assert_eq!(
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash_two, &5u32, &ctx.developer_two, &bounty_b)
            .err().unwrap(),
        Ok(Error::BountyAlreadyClaimed)
    );

    // Pool reflects both deductions.
    assert_eq!(ctx.client().milestone_balance(), pool_size - bounty_a - bounty_b);

    // Clawback returns only the unallocated remainder.
    let before = ctx.token_client().balance(&ctx.maintainer);
    ctx.advance_to_expiry();
    ctx.client().clawback_expired_funds(&ctx.maintainer);
    assert_eq!(ctx.token_client().balance(&ctx.maintainer) - before, pool_size - bounty_a - bounty_b);
    assert_eq!(ctx.client().milestone_balance(), 0);
}

/// End-to-end: full milestone lifecycle from creation through
/// multiple bounty releases to clawback of remaining funds.
#[test]
fn test_full_lifecycle_happy_path() {
    let ctx = TestContext::new();
    let pool_size = 100_000_000_000u128;
    let issues: Vec<(u32, u128)> =
        vec![(1, 10_000_000_000), (2, 20_000_000_000), (3, 15_000_000_000), (4, 25_000_000_000), (5, 5_000_000_000)];
    let total_bounties: u128 = issues.iter().map(|(_, a)| a).sum();
    let expected_remaining = pool_size - total_bounties;

    // === Phase 1: Create Pool ===
    ctx.fund_pool(pool_size);

    let info = ctx.client().milestone_info();
    assert!(info.is_some());
    let pool = info.unwrap();
    assert_eq!(pool.total_funds, pool_size);
    assert_eq!(pool.allocated_funds, 0);
    assert_eq!(ctx.client().milestone_balance(), pool_size);

    // === Phase 2: Release Bounties ===
    for (issue_id, amount) in &issues {
        ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, issue_id, &ctx.developer, amount);
        assert!(ctx.client().is_claimed(&ctx.repo_hash, issue_id));
    }

    assert_eq!(ctx.client().milestone_balance(), expected_remaining);
    assert_eq!(ctx.token_client().balance(&ctx.developer), total_bounties);

    // === Phase 3: Duplicate claims rejected ===
    for (issue_id, _) in &issues {
        let result = ctx.client().try_release_issue_bounty(
            &ctx.maintainer,
            &ctx.repo_hash,
            issue_id,
            &ctx.developer,
            &1_000_000_000,
        );
        assert!(result.is_err());
    }

    // === Phase 4: Clawback After Expiry ===
    let maintainer_before = ctx.token_client().balance(&ctx.maintainer);
    ctx.advance_to_expiry();
    ctx.client().clawback_expired_funds(&ctx.maintainer);
    let maintainer_after = ctx.token_client().balance(&ctx.maintainer);

    assert_eq!(maintainer_after - maintainer_before, expected_remaining);
    assert_eq!(ctx.client().milestone_balance(), 0);
}

/// Edge case: empty milestone (all funds clawed back, no claims).
#[test]
fn test_full_lifecycle_no_claims() {
    let ctx = TestContext::new();
    let pool_size = 50_000_000_000u128;

    ctx.fund_pool(pool_size);
    assert_eq!(ctx.client().milestone_balance(), pool_size);

    ctx.advance_to_expiry();

    let maintainer_before = ctx.token_client().balance(&ctx.maintainer);
    ctx.client().clawback_expired_funds(&ctx.maintainer);
    let maintainer_after = ctx.token_client().balance(&ctx.maintainer);

    assert_eq!(maintainer_after - maintainer_before, pool_size);
    assert_eq!(ctx.client().milestone_balance(), 0);
}

/// Edge case: pool fully claimed, clawback returns nothing.
#[test]
fn test_full_lifecycle_all_claimed() {
    let ctx = TestContext::new();
    let pool_size = 50_000_000_000u128;

    ctx.fund_pool(pool_size);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &pool_size);

    assert_eq!(ctx.client().milestone_balance(), 0);
    assert_eq!(ctx.token_client().balance(&ctx.developer), pool_size);

    ctx.advance_to_expiry();

    let result = ctx.client().try_clawback_expired_funds(&ctx.maintainer);
    assert!(result.is_err());
}
