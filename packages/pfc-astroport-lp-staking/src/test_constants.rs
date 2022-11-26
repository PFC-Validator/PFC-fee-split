use cosmwasm_std::testing::mock_info;
use cosmwasm_std::MessageInfo;

pub const DEFAULT_SENDER: &str = "terra1sq9ppsvt4k378wwhvm2vyfg7kqrhtve8p0n3a6";
pub const CONTRACT_CREATOR: &str = "terra16m3runusa9csfev7ymj62e8lnswu8um29k5zky";
pub const REWARD_TOKEN: &str = "terra1xj49zyqrwpv5k928jwfpfy2ha668nwdgkwlrg3";

pub fn default_sender() -> MessageInfo {
    mock_info(DEFAULT_SENDER, &[])
}

pub fn contract_creator() -> MessageInfo {
    mock_info(CONTRACT_CREATOR, &[])
}

pub mod liquidity {
    use crate::test_constants::REWARD_TOKEN;
    use crate::test_utils::mock_env_contract;
    use cosmwasm_std::{Env, Uint128};

    pub const LIQUIDITY: &str = "terra1l7xu2rl3c7qmtx3r5sd2tz25glf6jh8ul7aag7";

    pub const LP_REWARD_TOKEN: &str = REWARD_TOKEN;
    //  pub const LP_PAIR_TOKEN: &str = "terra17n5sunn88hpy965mzvt3079fqx3rttnplg779g";
    pub const LP_LIQUIDITY_TOKEN: &str = "terra1627ldjvxatt54ydd3ns6xaxtd68a2vtyu7kakj";
    pub const LP_DISTRIBUTION_SCHEDULE1: (u64, u64, Uint128) = (0, 100, Uint128::new(1000000u128));
    pub const LP_DISTRIBUTION_SCHEDULE2: (u64, u64, Uint128) =
        (100, 200, Uint128::new(10000000u128));

    pub fn lp_env() -> Env {
        let mut env = mock_env_contract(LIQUIDITY);
        env.block.height = 0;
        env
    }
}
