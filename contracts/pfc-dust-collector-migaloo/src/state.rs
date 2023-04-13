use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

use cw_storage_plus::{Item, Map};

pub(crate) const CONFIG_KEY: &str = "config_001";
pub(crate) const DENOM_KEY: &str = "denom_001";

pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
pub const ASSET_HOLDINGS: Map<String, Uint128> = Map::new(DENOM_KEY);

#[cw_serde]
pub struct Config {
    //   pub this: Addr,
    pub token_router: Addr,
    pub base_denom: String,
    pub return_contract: Addr,
}
