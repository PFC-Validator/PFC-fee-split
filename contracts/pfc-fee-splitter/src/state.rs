use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use pfc_fee_split::fee_split_msg::AllocationHolding;

pub(crate) const CONFIG_KEY: &str = "config_001";
pub(crate) const FEE_KEY: &str = "fees_001";

pub const ADMIN: Admin = Admin::new("admin");

pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
pub const ALLOCATION_HOLDINGS: Map<String, AllocationHolding> = Map::new(FEE_KEY);

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    pub this: Addr,
    pub owner: Addr,
    pub gov_contract: Addr,
}
