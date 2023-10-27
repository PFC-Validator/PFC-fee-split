use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Empty, Uint128};
use cw_ownable_derive::{cw_ownable_execute, cw_ownable_query};
use kujira::Denom;

use pfc_dust_collector_derive::pfc_dust_collect;
use pfc_whitelist::Whitelist;
use pfc_whitelist_derive::{pfc_whitelist_exec, pfc_whitelist_query};
#[cw_serde]
pub struct CollectorResponse<T> {
    pub entries: Vec<T>,
}
#[cw_serde]
pub struct Stage {
    pub address: Addr,
    pub denom: Denom,
}

#[pfc_dust_collect]
#[pfc_whitelist_exec]
#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// get some dust
    SetTokenRouter { contract: String },
    /// Set Base denom
    SetBaseDenom { denom: Denom },
    /// Change the number of funds/swaps we can do at a time. ADMIN ONLY
    SetMaxSwaps { max_swaps: u64 },
    /// minimum of zero
    SetAssetMinimum { denom: Denom, minimum: Uint128 },
    /// set the route path to exchange denom 'X' into something else.
    SetAssetStages {
        denom: Denom,
        stages: Vec<Vec<Stage>>,
    },
    /// passing this asset moving forward will just hold it, and not attempt to convert it. a 'Flush' will send it back (to avoid loops)
    ClearAsset { denom: Denom },
}

#[cw_ownable_query]
#[pfc_whitelist_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(CollectorResponse<AssetHolding>)]
    Assets {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(AssetHolding)]
    Asset { denom: Denom },
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub token_router: String,
    pub base_denom: Denom,
    pub return_contract: String,
    pub max_swaps: u64,
}
#[cw_serde]
pub struct AssetMinimum {
    pub denom: Denom,
    pub minimum: Uint128,
}
#[cw_serde]
pub struct AssetHolding {
    pub denom: Denom,
    pub minimum: Uint128, // only send $ after we have this amount in this coin
    pub balance: Uint128,
    pub stages: Vec<Vec<(Addr, Denom)>>,
}
#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub token_router: String,
    pub return_contract: String,
    pub base_denom: Denom,
    pub assets: Vec<AssetMinimum>,
    pub max_swaps: u64,
    pub flush_whitelist: Vec<Whitelist>,
    pub init_hook: Option<InitHook>,
}
/// Hook to be called after contract initialization
#[cw_serde]
pub struct InitHook {
    pub msg: Binary,
    pub contract_addr: String,
}

pub type MigrateMsg = Empty;
