use cosmwasm_std::{to_binary, Addr, Binary, Coin, CosmosMsg, StdResult, WasmMsg};
//use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum SendType {
    Wallet { receiver: Addr },
    SteakRewards { steak: Addr, receiver: Addr },
}
impl ToString for SendType {
    fn to_string(&self) -> String {
        match &self {
            SendType::Wallet { receiver } => format!("Wallet -> {}", receiver),
            SendType::SteakRewards { steak, receiver } => {
                format!("Steak:{} -> {}", steak, receiver)
            }
        }
    }
}
impl SendType {
    pub fn verify(&self, address:&Addr ) -> bool {
        match &self {
            SendType::Wallet { receiver } => receiver != address,
            SendType::SteakRewards {  receiver,.. } => receiver != address,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct AllocationDetail {
    pub name: String, // user-friendly name of wallet
    //   pub contract: String,    // contract/wallet to send too
    pub allocation: u8,      // what portion should we send
    pub send_after: Coin,    // only send $ after we have this amount in this coin
    pub send_type: SendType, // type of contract/wallet this is
}
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct AllocationHolding {
    pub name: String, // user-friendly name of wallet
    // pub contract: Addr,      // contract/wallet to send too
    pub allocation: u8,      // what portion should we send
    pub send_after: Coin,    // only send $ after we have this amount in this coin
    pub send_type: SendType, // type of contract/wallet this is
    pub balance: Vec<Coin>,
}
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub name: String,
    pub gov_contract: String,
    pub allocation: Vec<AllocationDetail>,
    // custom_params
    pub init_hook: Option<InitHook>,
}
/// Hook to be called after token initialization
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InitHook {
    pub msg: Binary,
    pub contract_addr: String,
}
/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]

pub enum ExecuteMsg {
    /// what other contracts will call to start the fly-wheel or fee distribution
    Deposit { flush: bool },

    AddAllocationDetail {
        name: String,
        allocation: u8,
        send_after: Coin,
        send_type: SendType,
    },
    // Modifies the fee, but does not send balance
    ModifyAllocationDetail {
        name: String,
        allocation: u8,
        send_after: Coin,
        send_type: SendType,
    },
    /// Removes the 'fee', sending whatever balance is there over
    RemoveAllocationDetail { name: String },
    /// Queries tokens held, and then re-assigns them to allocations, wiping out whatever was there.
    /// This is a ADMIN only function (must be called by current gov_contract)
    Reconcile {},
    /// Transfer gov-contract to another account; will not take effect unless the new owner accepts
    TransferGovContract { gov_contract: String, blocks: u64 },
    /// Accept an gov-contract transfer
    AcceptGovContract {},
    /// allow this address to flush funds
    AddToFlushWhitelist { address: String },
    /// remove this address from flush funds whitelist
    RemoveFromFlushWhitelist { address: String },
}
impl ExecuteMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        //  let msg = CollectablesExecuteMsg(self);
        to_binary(&self)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg<T: Into<String>, C>(
        self,
        contract_addr: T,
        funds: Vec<Coin>,
    ) -> StdResult<CosmosMsg<C>>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema,
    {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg,
            funds,
        };
        Ok(execute.into())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// lists all fees
    /// Return Type: AllocationResponse
    Allocations {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns allocation with name 'name'
    /// Return Type: AllocationHolding
    Allocation { name: String },
    /// returns ownership
    Ownership {},
    /// returns list of addresses allowed to flush
    FlushWhitelist {},
}
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct OwnershipResponse {
    pub owner: String,
    pub new_owner: Option<String>,
    pub block_height: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct WhitelistResponse {
    pub allowed: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct AllocationResponse {
    pub allocations: Vec<AllocationHolding>,
}
