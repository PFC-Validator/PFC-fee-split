use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod execute_msgs;
pub mod query_msgs;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct TokenBalance {
    pub amount: Uint128,
    pub token: String,
}
