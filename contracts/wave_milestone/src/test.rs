#![cfg(test)]

extern crate std;

use crate::types::Error;
use crate::{WaveMilestoneContract, WaveMilestoneContractClient};
use soroban_sdk::{
    contract, contractimpl, contracttype,
    testutils::{Address as _, Ledger},
    Address, BytesN, Env,
};

// ─────────────────────────────────────────────────────────────
// Mock Contracts
// ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
enum MockGuardKey {
    Maintainer(Address),
}

#[contract]
struct MockWaveGuard;

#[contractimpl]
impl MockWaveGuard {
    pub fn is_maintainer(env: Env, address: Address) -> bool {
        env.storage().instance().get::<_, bool>(&MockGuardKey::Maintainer(address)).unwrap_or(false)
    }

    pub fn add_maintainer(env: Env, address: Address) {
        env.storage().instance().set(&MockGuardKey::Maintainer(address), &true);
    }

    /// Revokes maintainer status — models a maintainer being removed from
    /// the WaveGuard registry (e.g. after going rogue or losing access).
    pub fn remove_maintainer(env: Env, address: Address) {
        env.storage()
            .instance()
            .set(&MockGuardKey::Maintainer(address), &false);
    }
}

#[contracttype]
#[derive(Clone)]
enum MockTokenKey {
    Balance(Address),
    Admin,
}

#[contract]
struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn init(env: Env, admin: Address) {
        env.storage().instance().set(&MockTokenKey::Admin, &admin);
    }

    pub fn mint(env: Env, to: Address, amount: u128) {
        let bal = env.storage().instance().get::<_, u128>(&MockTokenKey::Balance(to.clone())).unwrap_or(0);
        env.storage().instance().set(&MockTokenKey::Balance(to), &(bal + amount));
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: u128) {
        from.require_auth();
        let from_bal = env.storage().instance().get::<_, u128>(&MockTokenKey::Balance(from.clone())).unwrap_or(0);
        assert!(from_bal >= amount, "insufficient balance");
        let to_bal = env.storage().instance().get::<_, u128>(&MockTokenKey::Balance(to.clone())).unwrap_or(0);
        env.storage().instance().set(&MockTokenKey::Balance(from), &(from_bal - amount));
        env.storage().instance().set(&MockTokenKey::Balance(to), &(to_bal + amount));
    }

    pub fn balance(env: Env, id: Address) -> u128 {
        env.storage().instance().get::<_, u128>(&MockTokenKey::Balance(id)).unwrap_or(0)
    }
}

// ─────────────────────────────────────────────────────────────
// Test Helpers
// ─────────────────────────────────────────────────────────────

struct TestEnv {
    env: Env,
    maintainer: Address,
    developer: Address,
    stranger: Address,
    contract_id: Address,
    guard_id: Address,
    token_id: Address,
    repo_hash: BytesN<32>,
    expiry: u64,
}

fn setup() -> TestEnv {
    let env = Env::default();
    env.mock_all_auths();

    let maintainer = Address::generate(&env);
    let developer = Address::generate(&env);
    let stranger = Address::generate(&env);

    let guard_id = env.register(MockWaveGuard, ());
    MockWaveGuardClient::new(&env, &guard_id).add_maintainer(&maintainer);

    let token_id = env.register(MockToken, ());
    MockTokenClient::new(&env, &token_id).init(&maintainer);

    let contract_id = env.register(WaveMilestoneContract, ());

    let repo_hash = BytesN::from_array(&env, &[1u8; 32]);
    let expiry = env.ledger().timestamp() + 2_592_000;

    TestEnv { env, maintainer, developer, stranger, contract_id, guard_id, token_id, repo_hash, expiry }
}

fn fund_pool(t: &TestEnv, amount: u128) {
    MockTokenClient::new(&t.env, &t.token_id).mint(&t.maintainer, &amount);
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).create_milestone_pool(
        &t.maintainer,
        &t.guard_id,
        &t.token_id,
        &amount,
        &t.expiry,
    );
}

// ─────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────

#[test]
fn test_create_milestone_pool_success() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;

    MockTokenClient::new(&t.env, &t.token_id).mint(&t.maintainer, &pool_size);
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).create_milestone_pool(
        &t.maintainer,
        &t.guard_id,
        &t.token_id,
        &pool_size,
        &t.expiry,
    );

    let balance = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(balance, pool_size);

    let info = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_info();
    assert!(info.is_some());
    let pool = info.unwrap();
    assert_eq!(pool.total_funds, pool_size);
    assert_eq!(pool.allocated_funds, 0);
    assert_eq!(pool.maintainer, t.maintainer);
    assert_eq!(pool.asset, t.token_id);
    assert_eq!(pool.guard_contract, t.guard_id);
}

#[test]
fn test_create_pool_rejects_zero_amount() {
    let t = setup();

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_create_milestone_pool(
        &t.maintainer,
        &t.guard_id,
        &t.token_id,
        &0u128,
        &t.expiry,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::InvalidAmount));
}

#[test]
fn test_create_pool_rejects_unauthorized_maintainer() {
    let t = setup();
    let pool_size: u128 = 5_000_000_000;

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_create_milestone_pool(
        &t.stranger,
        &t.guard_id,
        &t.token_id,
        &pool_size,
        &t.expiry,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_release_bounty_success() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let bounty: u128 = 2_500_000_000;

    fund_pool(&t, pool_size);

    let developer_balance_before = MockTokenClient::new(&t.env, &t.token_id).balance(&t.developer);

    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &bounty,
    );

    let developer_balance_after = MockTokenClient::new(&t.env, &t.token_id).balance(&t.developer);
    assert_eq!(developer_balance_after - developer_balance_before, bounty);

    let remaining = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(remaining, pool_size - bounty);

    assert!(WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32));
}

#[test]
fn test_duplicate_claim_rejected() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let bounty: u128 = 2_500_000_000;

    fund_pool(&t, pool_size);

    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &bounty,
    );

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &bounty,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::BountyAlreadyClaimed));
}

#[test]
fn test_insufficient_pool_balance_graceful_revert() {
    let t = setup();
    let pool_size: u128 = 1_000_000_000;
    let bounty: u128 = 2_500_000_000;

    fund_pool(&t, pool_size);

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &bounty,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::InsufficientPoolBalance));

    // Pool should be intact
    let remaining = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(remaining, pool_size);
}

#[test]
fn test_over_allocation_after_partial_claims() {
    let t = setup();
    let pool_size: u128 = 5_000_000_000;
    let first_bounty: u128 = 3_000_000_000;
    let second_bounty: u128 = 3_000_000_000;

    fund_pool(&t, pool_size);

    // First claim should succeed
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &first_bounty,
    );

    // Second claim exceeds remaining 2B
    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &2u32,
        &t.developer,
        &second_bounty,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::InsufficientPoolBalance));
}

#[test]
fn test_clawback_expired_funds() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let bounty: u128 = 2_500_000_000;

    fund_pool(&t, pool_size);

    // Claim one issue
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &bounty,
    );

    // Jump past expiry
    t.env.ledger().set_timestamp(t.expiry + 1);

    let maintainer_balance_before = MockTokenClient::new(&t.env, &t.token_id).balance(&t.maintainer);

    WaveMilestoneContractClient::new(&t.env, &t.contract_id).clawback_expired_funds(&t.maintainer);

    let maintainer_balance_after = MockTokenClient::new(&t.env, &t.token_id).balance(&t.maintainer);
    let expected_clawback = pool_size - bounty;
    assert_eq!(maintainer_balance_after - maintainer_balance_before, expected_clawback);

    // Pool should show no remaining spendable balance
    let remaining = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(remaining, 0);
}

#[test]
fn test_clawback_before_expiry_rejected() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;

    fund_pool(&t, pool_size);

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_clawback_expired_funds(&t.maintainer);

    assert_eq!(result.err().unwrap(), Ok(Error::ClawbackTooEarly));
}

#[test]
fn test_unauthorized_caller_rejected() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;

    fund_pool(&t, pool_size);

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_clawback_expired_funds(&t.stranger);

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_non_maintainer_cannot_create_pool() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;

    MockTokenClient::new(&t.env, &t.token_id).mint(&t.stranger, &pool_size);

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_create_milestone_pool(
        &t.stranger,
        &t.guard_id,
        &t.token_id,
        &pool_size,
        &t.expiry,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
}

#[test]
fn test_multiple_issues_different_repos_independent() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let repo_b = BytesN::from_array(&t.env, &[2u8; 32]);

    fund_pool(&t, pool_size);

    // Claim issue 1 in repo_a
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &1_000_000_000,
    );

    // Claim same issue number in different repo should work
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &repo_b,
        &1u32,
        &t.developer,
        &2_000_000_000,
    );

    // Each should be independently tracked
    assert!(WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32));
    assert!(WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&repo_b, &1u32));
}

#[test]
fn test_pool_not_found_returns_none() {
    let t = setup();

    let info = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_info();
    assert!(info.is_none());

    let balance = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(balance, 0);
}

#[test]
fn test_release_issue_bounty_pool_not_found() {
    let t = setup();

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &1_000_000_000,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::PoolNotFound));
}

#[test]
fn test_wrong_maintainer_cannot_release_bounty() {
    let t = setup();
    fund_pool(&t, 10_000_000_000);

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
        &t.stranger,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &1_000_000_000,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));
    assert_eq!(WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance(), 10_000_000_000);
}

// ─────────────────────────────────────────────────────────────
// Composite Key & Claim Persistence (CM-01)
// ─────────────────────────────────────────────────────────────

/// The claim key is the composite `(repo_hash, issue_id)` — both components
/// are required to identify a claim.  This test verifies that changing either
/// component produces a distinct claim slot.
#[test]
fn test_composite_key_scopes_claims() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let repo_b = BytesN::from_array(&t.env, &[2u8; 32]);

    fund_pool(&t, pool_size);

    // Claim: (repo_hash, issue_id=1)
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &1_000_000_000,
    );

    // Same repo, different issue — independent claim
    assert!(!WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &2u32));
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &2u32,
        &t.developer,
        &1_000_000_000,
    );

    // Different repo, same issue — independent claim
    assert!(!WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&repo_b, &1u32));
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &repo_b,
        &1u32,
        &t.developer,
        &1_000_000_000,
    );

    // All three claim slots are independently tracked
    assert!(WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32));
    assert!(WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &2u32));
    assert!(WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&repo_b, &1u32));
}

/// Claim records stored in Persistent storage survive indefinite ledger
/// advancement.  Temporary storage entries would be pruned after their TTL
/// elapses; this confirms the CM-01 fix is effective.
#[test]
fn test_claim_persists_after_ledger_advancement() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let bounty: u128 = 2_500_000_000;

    fund_pool(&t, pool_size);

    // Claim an issue
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &bounty,
    );

    assert!(WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32));

    // Advance the ledger by a massive amount — well past any Temporary TTL
    t.env.ledger().set_timestamp(t.expiry + 10_000_000);

    // The claim record must still be visible in Persistent storage
    assert!(
        WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32),
        "claim must persist in storage after significant ledger advancement"
    );

    // Duplicate claim must still be rejected
    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &bounty,
    );
    assert_eq!(
        result.err().unwrap(),
        Ok(Error::BountyAlreadyClaimed),
        "duplicate-claim guard must remain active after ledger advancement"
    );
}

// ─────────────────────────────────────────────────────────────
// Malicious / Rogue Maintainer Scenarios (Issue #110)
// ─────────────────────────────────────────────────────────────
//
// A maintainer is a privileged actor: they fund pools, release bounties,
// and reclaim expired escrow. These tests pin down what a *rogue* maintainer
// — or one whose privileges have been revoked — can and cannot do, so any
// future change to the authorization model is caught by CI.

/// A maintainer removed from the WaveGuard registry can no longer release
/// new bounties: the on-chain `is_maintainer` check must be re-evaluated on
/// every call, not cached at pool-creation time.
#[test]
fn test_revoked_maintainer_cannot_release_bounty() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    fund_pool(&t, pool_size);

    // Maintainer goes rogue and is stripped of registry access.
    MockWaveGuardClient::new(&t.env, &t.guard_id).remove_maintainer(&t.maintainer);

    let result =
        WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
            &t.maintainer,
            &t.repo_hash,
            &1u32,
            &t.developer,
            &1_000_000_000,
        );

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedMaintainer));

    // Pool must be untouched — no funds leaked.
    let remaining = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(remaining, pool_size);
}

/// A maintainer removed from the WaveGuard registry can no longer claw
/// back expired funds — clawback now requires active registry membership
/// in addition to pool ownership.
#[test]
fn test_revoked_maintainer_cannot_clawback() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    fund_pool(&t, pool_size);

    MockWaveGuardClient::new(&t.env, &t.guard_id).remove_maintainer(&t.maintainer);
    t.env.ledger().set_timestamp(t.expiry + 1);

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id)
        .try_clawback_expired_funds(&t.maintainer);

    assert_eq!(after - before, pool_size);
}

/// A second, separately-authorized maintainer (a colluding or rogue
/// co-maintainer) must NOT be able to claw back a pool they did not create.
/// Clawback is restricted to the exact `pool.maintainer` address.
#[test]
fn test_rogue_co_maintainer_cannot_clawback_others_pool() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    fund_pool(&t, pool_size);

    // `stranger` is promoted to a registry maintainer, but is not the pool owner.
    MockWaveGuardClient::new(&t.env, &t.guard_id).add_maintainer(&t.stranger);
    t.env.ledger().set_timestamp(t.expiry + 1);

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id)
        .try_clawback_expired_funds(&t.stranger);

    assert_eq!(result.err().unwrap(), Ok(Error::UnauthorizedCaller));

    // Escrow stays put for the rightful owner.
    let remaining = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(remaining, pool_size);
}

/// Characterization: bounty release is authorized by WaveGuard membership,
/// NOT by pool ownership. Any address the registry trusts as a maintainer can
/// release from the pool. This documents the shared-maintainer trust model so
/// that tightening it (e.g. restricting release to `pool.maintainer`) is a
/// deliberate, test-visible change rather than a silent regression.
#[test]
fn test_co_maintainer_can_release_bounty() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let bounty: u128 = 1_000_000_000;
    fund_pool(&t, pool_size);

    MockWaveGuardClient::new(&t.env, &t.guard_id).add_maintainer(&t.stranger);

    // A different maintainer than the pool creator releases the bounty.
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.stranger,
        &t.repo_hash,
        &7u32,
        &t.developer,
        &bounty,
    );

    let remaining = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(remaining, pool_size - bounty);
}

/// Characterization / self-dealing: the contract does not prevent a
/// maintainer from naming themselves as the `developer` and paying the bounty
/// to their own address. On-chain there is no separation-of-duties guard;
/// trust is delegated to WaveGuard governance. This test makes that explicit
/// so a future on-chain mitigation is caught.
#[test]
fn test_maintainer_self_payout_is_not_blocked() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    let bounty: u128 = 4_000_000_000;
    fund_pool(&t, pool_size);

    let before = MockTokenClient::new(&t.env, &t.token_id).balance(&t.maintainer);

    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.maintainer, // developer == maintainer
        &bounty,
    );

    let after = MockTokenClient::new(&t.env, &t.token_id).balance(&t.maintainer);
    assert_eq!(after - before, bounty);
}

/// A maintainer cannot drain more than they deposited by calling clawback
/// twice: the first call zeroes the spendable balance, the second is rejected.
#[test]
fn test_double_clawback_rejected() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    fund_pool(&t, pool_size);

    t.env.ledger().set_timestamp(t.expiry + 1);
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).clawback_expired_funds(&t.maintainer);

    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id)
        .try_clawback_expired_funds(&t.maintainer);

    assert_eq!(result.err().unwrap(), Ok(Error::NoFundsToClawback));
}

// ─────────────────────────────────────────────────────────────
// Issue #61 — completed flag is set only on successful release
// ─────────────────────────────────────────────────────────────

/// `is_claimed` must return `true` after a successful bounty release.
#[test]
fn test_completed_flag_set_after_successful_release() {
    let t = setup();
    fund_pool(&t, 10_000_000_000);

    assert!(!WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32));

    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &1_000_000_000,
    );

    assert!(WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32));
}

/// A failed release attempt (amount exceeds pool) must NOT set the claim —
/// `is_claimed` must still return `false` so the issue can be retried.
#[test]
fn test_completed_flag_not_set_on_failed_release() {
    let t = setup();
    fund_pool(&t, 1_000_000_000);

    // Attempt to release more than the pool holds — this must fail.
    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &5_000_000_000,
    );
    assert_eq!(result.err().unwrap(), Ok(Error::InsufficientPoolBalance));

    // Claim must be absent — the issue should still be releasable.
    assert!(!WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32));

    // Confirm a correct-sized release now succeeds.
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &500_000_000,
    );
    assert!(WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32));
}

/// After reclaiming the escrow via clawback, a maintainer must not be able to
/// keep paying out bounties — the spendable balance is gone.
#[test]
fn test_no_release_after_clawback_drains_pool() {
    let t = setup();
    let pool_size: u128 = 10_000_000_000;
    fund_pool(&t, pool_size);

    t.env.ledger().set_timestamp(t.expiry + 1);
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).clawback_expired_funds(&t.maintainer);

    let result =
        WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
            &t.maintainer,
            &t.repo_hash,
            &1u32,
            &t.developer,
            &1u128,
        );

    assert_eq!(result.err().unwrap(), Ok(Error::InsufficientPoolBalance));
}

/// Boundary: a maintainer may release exactly the remaining balance, but not
/// one unit more. Confirms the pool cannot be over-drawn across issues.
#[test]
fn test_drain_pool_to_zero_then_release_fails() {
    let t = setup();
    let pool_size: u128 = 5_000_000_000;
    fund_pool(&t, pool_size);

    // Release the entire pool in one bounty (amount == remaining is allowed).
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &pool_size,
    );

    let remaining = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(remaining, 0);

    // Any further payout, even the smallest unit, must be rejected.
    let result =
        WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
            &t.maintainer,
            &t.repo_hash,
            &2u32,
            &t.developer,
            &1u128,
        );

    assert_eq!(result.err().unwrap(), Ok(Error::InsufficientPoolBalance));
}

/// Characterization of a robustness gap: `create_milestone_pool` overwrites
/// any existing pool without an "already initialized" guard. A maintainer can
/// re-create the pool, which resets `allocated_funds` to zero and replaces the
/// stored accounting. Prior bounty allocations are forgotten even though those
/// tokens already left the vault.
///
/// This test pins the *current* behavior. If an init-guard
/// (e.g. `Error::PoolAlreadyExists`) is later added, update this test — the
/// failure is the signal that the gap was closed.
#[test]
fn test_recreate_pool_overwrites_existing_accounting() {
    let t = setup();
    let first_size: u128 = 10_000_000_000;
    let bounty: u128 = 3_000_000_000;
    fund_pool(&t, first_size);

    // A real bounty is paid out, so allocated_funds advances.
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &bounty,
    );

    // The maintainer re-creates the pool with a fresh (smaller) deposit.
    let second_size: u128 = 1_000_000_000;
    MockTokenClient::new(&t.env, &t.token_id).mint(&t.maintainer, &second_size);
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).create_milestone_pool(
        &t.maintainer,
        &t.guard_id,
        &t.token_id,
        &second_size,
        &t.expiry,
    );

    // Accounting was clobbered: allocation reset to 0 and balance now reflects
    // only the second deposit, despite the earlier bounty having been paid.
    let pool = WaveMilestoneContractClient::new(&t.env, &t.contract_id)
        .milestone_info()
        .unwrap();
    assert_eq!(pool.total_funds, second_size);
    assert_eq!(pool.allocated_funds, 0);

    let remaining = WaveMilestoneContractClient::new(&t.env, &t.contract_id).milestone_balance();
    assert_eq!(remaining, second_size);
}

// ─────────────────────────────────────────────────────────────
// repo_hash Guard Rail Tests (Issue #105)
// ─────────────────────────────────────────────────────────────

/// An all-zero repo_hash is the canonical null/unset value and must be
/// rejected before any pool or storage lookup occurs.
#[test]
fn test_release_bounty_rejects_zero_repo_hash() {
    let t = setup();
    fund_pool(&t, 10_000_000_000);

    let zero_hash = BytesN::from_array(&t.env, &[0u8; 32]);
    let result = WaveMilestoneContractClient::new(&t.env, &t.contract_id).try_release_issue_bounty(
        &t.maintainer,
        &zero_hash,
        &1u32,
        &t.developer,
        &1_000_000_000,
    );

    assert_eq!(result.err().unwrap(), Ok(Error::InvalidRepoHash));
}

/// A non-zero repo_hash must pass validation and proceed normally.
#[test]
fn test_release_bounty_accepts_nonzero_repo_hash() {
    let t = setup();
    fund_pool(&t, 10_000_000_000);

    // t.repo_hash is [1u8; 32] — non-zero, must succeed
    WaveMilestoneContractClient::new(&t.env, &t.contract_id).release_issue_bounty(
        &t.maintainer,
        &t.repo_hash,
        &1u32,
        &t.developer,
        &1_000_000_000,
    );

    assert!(WaveMilestoneContractClient::new(&t.env, &t.contract_id).is_claimed(&t.repo_hash, &1u32));
}
