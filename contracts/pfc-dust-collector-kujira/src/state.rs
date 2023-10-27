//use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};
use kujira::Denom;
//use kujira::Denom;

pub(crate) const CONFIG_KEY: &str = "config_001";
pub(crate) const DENOM_KEY: &str = "denom_001";
pub(crate) const DENOM_STAGES: &str = "stages_001";

pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
/// denom - minimum amount to hold
pub const ASSET_HOLDINGS: Map<String, Uint128> = Map::new(DENOM_KEY);
pub const ASSET_STAGES: Map<String, Vec<Vec<(Addr, Denom)>>> = Map::new(DENOM_STAGES);

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    /// The address to send swap messages to
    pub token_router: Addr,
    /// The denom which we actually want
    pub base_denom: Denom,
    /// Where to send it
    pub return_contract: Addr,
    /// how many funds/swaps can we do at a single time
    pub max_swaps: u64,
}
