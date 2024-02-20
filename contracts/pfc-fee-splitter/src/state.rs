use cosmwasm_std::Addr;
use cw_controllers::Admin;
use cw_item_set::Set;
use cw_storage_plus::{Item, Map};
use pfc_fee_split::fee_split_msg::AllocationHolding;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) const CONFIG_KEY: &str = "config_002";
pub(crate) const FEE_KEY: &str = "fees_002";

pub(crate) const FLUSH_WHITELIST_KEY: &str = "flush_001";
pub(crate) const FLUSH_WHITELIST_COUNTER_KEY: &str = "flush_001";

pub const ADMIN: Admin = Admin::new("admin");

pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);
pub const ALLOCATION_HOLDINGS: Map<String, AllocationHolding> = Map::new(FEE_KEY);
pub const FLUSH_WHITELIST: Set<Addr> = Set::new(FLUSH_WHITELIST_KEY, FLUSH_WHITELIST_COUNTER_KEY);

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    pub this: Addr,
    pub gov_contract: Addr,
    pub new_gov_contract: Option<Addr>,
    pub change_gov_contract_by_height: Option<u64>,
}
