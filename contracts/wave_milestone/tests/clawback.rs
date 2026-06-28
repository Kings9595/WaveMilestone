mod common;

use common::*;
use soroban_sdk::{testutils::Events, TryFromVal};
use wave_milestone::events::FundsClawedBackEvent;
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

    assert_eq!(result.err().unwrap(), Ok(Error::ClawbackTooEarly));
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
fn test_clawback_event_emitted() {
    let ctx = TestContext::new();
    let pool_size = DEFAULT_POOL_FUNDS;
    let bounty = DEFAULT_BOUNTY;

    ctx.fund_pool(pool_size);
    ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, &1u32, &ctx.developer, &bounty);

    ctx.advance_to_expiry();
    let expected_return = pool_size - bounty;
    ctx.client().clawback_expired_funds(&ctx.maintainer);

    let events = ctx.env.events().all();

    // Find the clawback event by parsing each event's data as FundsClawedBackEvent
    let clawback_found = events.iter().any(|(_, _, data)| {
        FundsClawedBackEvent::try_from_val(&ctx.env, &data)
            .is_ok_and(|evt: FundsClawedBackEvent| {
                evt.maintainer == ctx.maintainer && evt.amount == expected_return
            })
    });
    assert!(clawback_found, "FundsClawedBackEvent with correct data not found in emitted events");
}

#[test]
fn test_clawback_after_all_bounties_claimed() {
    let ctx = TestContext::new();
    let pool_size = 100_000_000_000u128;
    let bounties = [(1u32, 20_000_000_000u128), (2, 30_000_000_000), (3, 25_000_000_000)];
    let total_claimed: u128 = bounties.iter().map(|(_, a)| a).sum();
    let expected_remaining = pool_size - total_claimed;

    ctx.fund_pool(pool_size);
    for (issue_id, amount) in &bounties {
        ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, issue_id, &ctx.developer, amount);
    }

    assert_eq!(ctx.client().milestone_balance(), expected_remaining);

    ctx.advance_to_expiry();

    let balance_before = ctx.token_client().balance(&ctx.maintainer);
    ctx.client().clawback_expired_funds(&ctx.maintainer);
    let balance_after = ctx.token_client().balance(&ctx.maintainer);

    assert_eq!(balance_after - balance_before, expected_remaining);
    assert_eq!(ctx.client().milestone_balance(), 0);
}

#[test]
fn test_clawback_on_empty_pool() {
    let ctx = TestContext::new();
    let pool_size = 100_000_000_000u128;
    let bounties = [(1u32, 40_000_000_000u128), (2, 35_000_000_000), (3, 25_000_000_000)];
    let total_claimed: u128 = bounties.iter().map(|(_, a)| a).sum();
    assert_eq!(total_claimed, pool_size);

    ctx.fund_pool(pool_size);
    for (issue_id, amount) in &bounties {
        ctx.client().release_issue_bounty(&ctx.maintainer, &ctx.repo_hash, issue_id, &ctx.developer, amount);
    }

    assert_eq!(ctx.client().milestone_balance(), 0);

    ctx.advance_to_expiry();

    let result = ctx.client().try_clawback_expired_funds(&ctx.maintainer);

    assert_eq!(result.err().unwrap(), Ok(Error::NoFundsToClawback));
}
