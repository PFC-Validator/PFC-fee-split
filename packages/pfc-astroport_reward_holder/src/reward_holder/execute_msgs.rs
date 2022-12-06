use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub token: String,
    pub name: String,
    pub lp_token: String,
    pub gov_contract: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Unbond {
        amount: Uint128,
    },
    /// Withdraw pending rewards
    Withdraw {},
    UpdateConfig {
        token: Option<String>,
        name: Option<String>,
        lp_token: Option<String>,
    },
    MigrateReward {
        recipient: String,
        amount: Uint128,
    },
    /// Transfer gov-contract to another account; will not take effect unless the new owner accepts
    TransferGovContract {
        gov_contract: String,
        blocks: u64,
    },
    /// Accept an gov-contract transfer
    AcceptGovContract {},
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    Bond {},
    Receive {},
}

/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
