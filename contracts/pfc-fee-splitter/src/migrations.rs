use crate::state::{Config, CONFIG_KEY};
use cosmwasm_std::{Addr, StdError, StdResult, Storage};

use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
const CONFIG_V100_KEY: &str = "config_001";
pub const CONFIG_V100: Item<ConfigV100> = Item::new(CONFIG_V100_KEY);

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct ConfigV100 {
    pub this: Addr,
    pub owner: Addr,
    pub gov_contract: Addr,
}
impl ConfigV100 {
    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        if CONFIG_V100_KEY == CONFIG_KEY {
            Err(StdError::generic_err(
                "PFC-Fee-Split: Migration Failed. Config keys are the same",
            ))
        } else {
            CONFIG_V100.load(storage)
        }
    }

    pub fn migrate_from(&self) -> Config {
        Config {
            this: self.this.clone(),
            gov_contract: self.gov_contract.clone(),
            new_gov_contract: None,
            change_gov_contract_by_height: None,
        }
    }
}
