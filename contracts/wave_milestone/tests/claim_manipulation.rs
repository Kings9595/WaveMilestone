/// Security audit tests — unauthorized claim manipulation (CM-01, CM-02)
///
/// These tests document and verify the contract's defences against the
/// claim-manipulation vectors identified during the security audit recorded
/// in `lib.rs` and `types.rs`.
mod common;

use common::*;
use soroban_sdk::testutils::Ledger;
use wave_milestone::types::Error;

// ── CM-01: Persistent storage duplicate-claim guard ──────────────────────────

/// A claimed issue must be permanently rejected on subsequent attempts.
///
/// Verifies that the duplicate-claim guard uses Persistent storage and
/// therefore survives for the contract's lifetime, regardless of any ledger
/// TTL that would prune a Temporary storage entry.
#[test]
fn test_claim_guard_is_durable_across_ledger_advance() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    // First claim succeeds
    ctx.client().release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &42u32,
        &ctx.developer,
        &DEFAULT_BOUNTY,
    );

    // Advance time well past any Temporary-storage TTL (simulate expiry)
    ctx.env.ledger().set_timestamp(ctx.expiry + 1_000_000);

    // Second attempt for the same (repo_hash, issue_id) must still be rejected
    let result = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &42u32,
        &ctx.developer,
        &DEFAULT_BOUNTY,
    );

    assert_eq!(
        result.err().unwrap(),
        Ok(Error::BountyAlreadyClaimed),
        "duplicate-claim guard must reject re-claim even after significant ledger advancement"
    );
}

/// A second maintainer re-trying the same claim after pool expiry is blocked.
///
/// Regression for CM-01: ensures the guard is not ledger-timestamp-dependent.
#[test]
fn test_claim_guard_survives_pool_expiry() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    ctx.client().release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &7u32,
        &ctx.developer,
        &DEFAULT_BOUNTY,
    );

    // Pool expires
    ctx.advance_to_expiry();

    // Attempt to re-claim the same issue after expiry
    let result = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &7u32,
        &ctx.developer,
        &DEFAULT_BOUNTY,
    );

    assert_eq!(
        result.err().unwrap(),
        Ok(Error::BountyAlreadyClaimed),
        "claim guard must hold after pool expiry"
    );
}

/// `is_claimed` view correctly reflects Persistent storage after the fix.
#[test]
fn test_is_claimed_reflects_persistent_storage() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    assert!(!ctx.client().is_claimed(&ctx.repo_hash, &1u32));

    ctx.client().release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &1u32,
        &ctx.developer,
        &DEFAULT_BOUNTY,
    );

    assert!(
        ctx.client().is_claimed(&ctx.repo_hash, &1u32),
        "is_claimed must return true after successful release"
    );

    // Claim for a different issue on the same repo must remain false
    assert!(
        !ctx.client().is_claimed(&ctx.repo_hash, &2u32),
        "is_claimed must return false for unclaimed issues"
    );
}

// ── CM-02: Developer address not restricted — expected / permitted behavior ───

/// Documents that a maintainer can direct a bounty to any address (CM-02).
///
/// This is an accepted design trade-off: the protocol is permissioned and
/// maintainers are vetted by WaveGuard.  The test records the expected,
/// permitted behavior so that any future restriction of this pattern is
/// a deliberate, reviewed change.
#[test]
fn test_maintainer_can_redirect_developer_address() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    let balance_before = ctx.token_client().balance(&ctx.stranger);

    // Maintainer deliberately redirects the bounty to `stranger` instead
    // of the developer who completed the work.
    ctx.client().release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &99u32,
        &ctx.stranger, // arbitrary address — permitted by design
        &DEFAULT_BOUNTY,
    );

    let balance_after = ctx.token_client().balance(&ctx.stranger);
    assert_eq!(
        balance_after - balance_before,
        DEFAULT_BOUNTY,
        "maintainer can direct bounty to any address (CM-02: accepted design trade-off)"
    );

    // Original developer receives nothing
    assert_eq!(
        ctx.token_client().balance(&ctx.developer),
        0,
        "developer not credited when maintainer redirects payment"
    );
}

/// A non-maintainer cannot manufacture a claim by passing themselves as
/// maintainer even if they know a valid (repo_hash, issue_id) pair.
#[test]
fn test_non_maintainer_cannot_claim_on_behalf() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    // Stranger attempts to release a bounty to themselves
    let result = ctx.client().try_release_issue_bounty(
        &ctx.stranger,
        &ctx.repo_hash,
        &1u32,
        &ctx.stranger,
        &DEFAULT_BOUNTY,
    );

    assert_eq!(
        result.err().unwrap(),
        Ok(Error::UnauthorizedMaintainer),
        "non-maintainer must not be able to release a bounty"
    );

    // Confirm no funds moved
    assert_eq!(ctx.token_client().balance(&ctx.stranger), 0);
    assert_eq!(ctx.client().milestone_balance(), DEFAULT_POOL_FUNDS);
}

/// Confirms a removed maintainer cannot retroactively re-claim an issue by
/// attempting to use a previously valid authority.
#[test]
fn test_revoked_maintainer_cannot_claim_after_removal() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);

    // Revoke maintainer status before any claim is made
    MockWaveGuardClient::new(&ctx.env, &ctx.guard_id).remove_maintainer(&ctx.maintainer);

    let result = ctx.client().try_release_issue_bounty(
        &ctx.maintainer,
        &ctx.repo_hash,
        &1u32,
        &ctx.developer,
        &DEFAULT_BOUNTY,
    );

    assert_eq!(
        result.err().unwrap(),
        Ok(Error::UnauthorizedMaintainer),
        "revoked maintainer must not be able to release a bounty"
    );
}
