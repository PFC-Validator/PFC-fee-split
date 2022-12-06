use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::vault::TokenBalance;
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
    pub token: String,
    pub name: String,
    pub lp_token: String,
    pub gov_contract: String,
    pub new_gov_contract: Option<String>,
    pub change_gov_contract_by_height: Option<u64>,
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
