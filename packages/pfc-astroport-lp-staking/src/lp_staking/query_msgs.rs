use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::lp_staking::TokenBalance;
use cosmwasm_std::Uint128;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    State {},
    StakerInfo { staker: String },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub admin: String,
    pub token: String,
    pub pair: String,
    pub lp_token: String,
    //    pub whitelisted_contracts: Vec<String>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub total_staked: Uint128,
    pub counters_per_token: Vec<TokenBalance>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct StakerInfoResponse {
    pub staker: String,
    pub total_staked: Uint128,
    pub estimated_rewards: Vec<TokenBalance>,
    pub last_claimed: Option<u64>,
}
