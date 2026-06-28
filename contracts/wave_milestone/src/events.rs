use soroban_sdk::{contracttype, Address, BytesN};

// ─────────────────────────────────────────────────────────────
// Contract Events
// ─────────────────────────────────────────────────────────────
// Events are emitted via env.events().publish() with topic symbols.
// These types serialize the event payload for off-chain indexers.

pub const TOPIC_POOL_CREATED: &str = "milestone_pool_created";
pub const TOPIC_BOUNTY_RELEASED: &str = "bounty_released";
pub const TOPIC_FUNDS_CLAWED_BACK: &str = "funds_clawed_back";

#[derive(Clone)]
#[contracttype]
pub struct PoolCreatedEvent {
    pub maintainer: Address,
    pub asset: Address,
    pub total_funds: u128,
    pub expiry: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct BountyReleasedEvent {
    pub repo_hash: BytesN<32>,
    pub issue_id: u32,
    pub developer: Address,
    pub amount: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub struct FundsClawedBackEvent {
    pub maintainer: Address,
    pub amount: u128,
}
