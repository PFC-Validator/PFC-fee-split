use cosmwasm_std::{Addr, Decimal};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod execute_msgs;
pub mod query_msgs;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct TokenBalance {
    pub amount: Decimal,
    pub token: Addr,
    pub last_block_rewards_seen: u64,
}
