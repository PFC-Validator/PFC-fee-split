use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw_ownable_derive::{cw_ownable_execute, cw_ownable_query};

use pfc_dust_collector_derive::pfc_dust_collect;

use pfc_whitelist_derive::{pfc_whitelist_exec, pfc_whitelist_query};
#[cw_serde]
pub struct CollectorResponse<T> {
    pub entries: Vec<T>,
}

#[pfc_dust_collect]
#[pfc_whitelist_exec]
#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// get some dust
    SetTokenRouter { contract: String },
    /// Set Base denom
    SetBaseDenom { denom: String },
    /// minimum of zero
    SetAssetMinimum { denom: String, minimum: Uint128 },
    /// passing this asset moving forward will just hold it, and not attempt to convert it. a 'Flush' will send it back (to avoid loops)
    ClearAsset { denom: String },
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
    Asset { denom: String },
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub token_router: String,
    pub base_denom: String,
    pub return_contract: String,
}
#[cw_serde]
pub struct AssetMinimum {
    pub denom: String,
    pub minimum: Uint128,
}
#[cw_serde]
pub struct AssetHolding {
    pub denom: String,
    pub minimum: Uint128, // only send $ after we have this amount in this coin
    pub balance: Uint128,
}
