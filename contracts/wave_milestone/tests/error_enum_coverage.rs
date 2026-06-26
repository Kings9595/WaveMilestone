mod common;

use common::*;
use wave_milestone::types::Error;

// ── PoolNotFound (1) ─────────────────────────────────────────

#[test]
fn test_error_pool_not_found_release() {
    let ctx = TestContext::new();
    let result =
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &1_000u128);
    assert_eq!(result.err().unwrap(), Ok(Error::PoolNotFound));
}

#[test]
fn test_error_pool_not_found_clawback() {
    let ctx = TestContext::new();
    let result = ctx.client().try_clawback_expired_funds(&ctx.maintainer);
    assert_eq!(result.err().unwrap(), Ok(Error::PoolNotFound));
}

// ── PoolNotExpired (2) ───────────────────────────────────────

#[test]
fn test_error_pool_not_expired() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);
    // Clawback before expiry — ledger is still before ctx.expiry
    let result = ctx.client().try_clawback_expired_funds(&ctx.maintainer);
    assert_eq!(result.err().unwrap(), Ok(Error::PoolNotExpired));
}

// ── BountyAlreadyClaimed (3) ─────────────────────────────────

#[test]
fn test_error_bounty_already_claimed() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);
    let result =
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);
    assert_eq!(result.err().unwrap(), Ok(Error::BountyAlreadyClaimed));
}

// ── InsufficientPoolBalance (4) ──────────────────────────────

#[test]
fn test_error_insufficient_pool_balance() {
    let ctx = TestContext::new();
    ctx.fund_pool(1_000u128);
    let result =
        ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &2_000u128);
    assert_eq!(result.err().unwrap(), Ok(Error::InsufficientPoolBalance));
}

// ── UnauthorizedMaintainer (5) ───────────────────────────────

#[test]
fn test_error_unauthorized_maintainer_create_pool() {
    let ctx = TestContext::new();
    ctx.token_client().mint(&ctx.stranger, &DEFAULT_POOL_FUNDS);
    let result = ctx.client().try_create_milestone_pool(
        &ctx.stranger,
        &ctx.guard_id,
        &ctx.token_id,
        &DEFAULT_POOL_FUNDS,
        &ctx.expiry,
    );
    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_error_unauthorized_maintainer_release_bounty() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);
    let result =
        ctx.client().try_release_issue_bounty(&ctx.stranger, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_BOUNTY);
    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

// ── UnauthorizedCaller (6) ───────────────────────────────────

#[test]
fn test_error_unauthorized_caller_clawback() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);
    ctx.advance_to_expiry();
    let result = ctx.client().try_clawback_expired_funds(&ctx.stranger);
    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedCaller));
}

// ── NoFundsToClawback (7) ────────────────────────────────────

#[test]
fn test_error_no_funds_to_clawback() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);
    // Drain the pool completely
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &DEFAULT_POOL_FUNDS);
    ctx.advance_to_expiry();
    let result = ctx.client().try_clawback_expired_funds(&ctx.maintainer);
    assert_eq!(result.err().unwrap(), Ok(Error::NoFundsToClawback));
}

// ── TransferFailed (8) ───────────────────────────────────────
// Not currently returned by the contract (reserved for future use).
// Assert the discriminant value is correct so enum layout is pinned.

#[test]
fn test_error_transfer_failed_discriminant() {
    assert_eq!(Error::TransferFailed as u32, 8);
}

// ── InvalidAmount (9) ────────────────────────────────────────

#[test]
fn test_error_invalid_amount_create_pool() {
    let ctx = TestContext::new();
    let result =
        ctx.client().try_create_milestone_pool(&ctx.maintainer, &ctx.guard_id, &ctx.token_id, &0u128, &ctx.expiry);
    assert_eq!(result.err().unwrap(), Ok(Error::InvalidAmount));
}

#[test]
fn test_error_invalid_amount_release_bounty() {
    let ctx = TestContext::new();
    ctx.fund_pool(DEFAULT_POOL_FUNDS);
    let result = ctx.client().try_release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &0u128);
    assert_eq!(result.err().unwrap(), Ok(Error::InvalidAmount));
}

// ── ExpiryInPast (10) ────────────────────────────────────────

#[test]
fn test_error_expiry_in_past() {
    let ctx = TestContext::new();
    ctx.token_client().mint(&ctx.maintainer, &DEFAULT_POOL_FUNDS);
    let past_expiry = ctx.env.ledger().timestamp(); // now == not in the future
    let result = ctx.client().try_create_milestone_pool(
        &ctx.maintainer,
        &ctx.guard_id,
        &ctx.token_id,
        &DEFAULT_POOL_FUNDS,
        &past_expiry,
    );
    assert_eq!(result.err().unwrap(), Ok(Error::ExpiryInPast));
}
