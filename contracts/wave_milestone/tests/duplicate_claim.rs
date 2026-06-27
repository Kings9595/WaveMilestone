mod common;

use common::*;
use wave_milestone::types::Error;

/// A duplicate claim attempt targeting a **different developer** must still be
/// rejected.  The guard key is `(repo_hash, issue_id)` — the developer address
/// is irrelevant to uniqueness.
#[test]
fn test_duplicate_claim_different_developer_rejected() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    let result = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &1u32,
        &ctx.developer_two, // different developer, same issue
        &DEFAULT_BOUNTY,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::BountyAlreadyClaimed));
    // developer_two must receive nothing
    assert_eq!(ctx.token_client().balance(&ctx.developer_two), 0);
}

/// A rejected duplicate claim must leave both the pool balance and the
/// original developer's balance completely unchanged.
#[test]
fn test_duplicate_claim_does_not_alter_balances() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &5u32, &ctx.developer, &DEFAULT_BOUNTY);

    let pool_after_first = ctx.client().milestone_balance();
    let dev_after_first = ctx.token_client().balance(&ctx.developer);

    // Attempt duplicate — must fail without moving any funds
    let _ = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &5u32,
        &ctx.developer,
        &DEFAULT_BOUNTY,
    );

    assert_eq!(ctx.client().milestone_balance(), pool_after_first);
    assert_eq!(ctx.token_client().balance(&ctx.developer), dev_after_first);
}

/// After claiming several distinct issues, every one of them must be
/// individually guarded against re-claim.
#[test]
fn test_multiple_sequential_claims_all_guarded() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    let issues: &[u32] = &[10, 20, 30];
    let per_bounty = DEFAULT_BOUNTY / 4;

    for &id in issues {
        ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &id, &ctx.developer, &per_bounty);
    }

    for &id in issues {
        let result = ctx.client().try_release_issue_bounty(
            &ctx.maintainer,
            &ctx.repo_hash,
            &id,
            &ctx.developer,
            &per_bounty,
        );
        assert_eq!(
            result.err().unwrap(),
            Ok(Error::BountyAlreadyClaimed),
            "issue {id} must be permanently guarded after its first claim"
        );
    }
}

#[test]
fn test_duplicate_claim_same_repo_same_issue_rejected() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    let result =
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    assert_eq!(result.err().unwrap(), Ok(Error::BountyAlreadyClaimed));
}

#[test]
fn test_duplicate_claim_different_issue_same_repo_allowed() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    // Same repo, different issue — should succeed
    let result =
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &2u32, &ctx.developer, &DEFAULT_BOUNTY);

    assert!(result.is_ok());
}

#[test]
fn test_duplicate_claim_same_issue_different_repo_allowed() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS * 2);

    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);

    // Different repo, same issue number — should succeed
    let result = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash_two,
        &1u32,
        &ctx.developer_two,
        &DEFAULT_BOUNTY,
    );

    assert!(result.is_ok());
}

#[test]
fn test_claim_status_is_scoped_by_repo_hash() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS * 2);
    let issue_id = 1u32;

    ctx.client().release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &issue_id,
        &ctx.developer,
        &DEFAULT_BOUNTY,
    );

    assert!(ctx.client().is_claimed(&ctx.repo_hash, &issue_id));
    assert!(!ctx.client().is_claimed(&ctx.repo_hash_two, &issue_id));

    ctx.client().release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash_two,
        &issue_id,
        &ctx.developer_two,
        &DEFAULT_BOUNTY,
    );

    assert!(ctx.client().is_claimed(&ctx.repo_hash, &issue_id));
    assert!(ctx.client().is_claimed(&ctx.repo_hash_two, &issue_id));
}
