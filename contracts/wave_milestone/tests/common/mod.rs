#![allow(dead_code)]

mod mock_guard;
mod mock_token;

pub use mock_guard::MockWaveGuardClient;
pub use mock_token::MockTokenClient;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env,
};
use wave_milestone::{WaveMilestoneContract, WaveMilestoneContractClient};

pub const DEFAULT_POOL_FUNDS: u128 = 10_000_000_000;
pub const DEFAULT_BOUNTY: u128 = 2_500_000_000;

pub struct TestContext {
    pub env: Env,
    pub maintainer: Address,
    pub developer: Address,
    pub developer_two: Address,
    pub stranger: Address,
    pub contract_id: Address,
    pub guard_id: Address,
    pub token_id: Address,
    pub repo_hash: BytesN<32>,
    pub repo_hash_two: BytesN<32>,
    pub expiry: u64,
}

impl TestContext {
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let maintainer = Address::generate(&env);
        let developer = Address::generate(&env);
        let developer_two = Address::generate(&env);
        let stranger = Address::generate(&env);

        let guard_id = env.register(mock_guard::MockWaveGuard, ());
        MockWaveGuardClient::new(&env, &guard_id).add_maintainer(&maintainer);

        let token_id = env.register(mock_token::MockToken, ());
        MockTokenClient::new(&env, &token_id).init(&maintainer);

        let contract_id = env.register(WaveMilestoneContract, ());

        let repo_hash = BytesN::from_array(&env, &[1u8; 32]);
        let repo_hash_two = BytesN::from_array(&env, &[2u8; 32]);
        let now = env.ledger().timestamp();
        let expiry = now + 2_592_000;

        Self {
            env,
            maintainer,
            developer,
            developer_two,
            stranger,
            contract_id,
            guard_id,
            token_id,
            repo_hash,
            repo_hash_two,
            expiry,
        }
    }

    pub fn fund_pool(&self, amount: u128) {
        MockTokenClient::new(&self.env, &self.token_id).mint(&self.maintainer, &amount);
        self.client().create_milestone_pool(
            &self.maintainer,
            &self.guard_id,
            &self.token_id,
            &amount,
            &self.expiry,
        );
    }

    pub fn client(&self) -> WaveMilestoneContractClient<'_> {
        WaveMilestoneContractClient::new(&self.env, &self.contract_id)
    }

    pub fn token_client(&self) -> MockTokenClient<'_> {
        MockTokenClient::new(&self.env, &self.token_id)
    }

    pub fn advance_to_expiry(&self) {
        self.env.ledger().set_timestamp(self.expiry + 1);
    }
}
