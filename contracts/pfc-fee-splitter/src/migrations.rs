use cosmwasm_std::{Addr, Api, Coin, DepsMut, Order, StdError, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use pfc_fee_split::fee_split_msg::{AllocationHolding, SendType};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Config, ALLOCATION_HOLDINGS, CONFIG_KEY};

const CONFIG_V100_KEY: &str = "config_001";
const FEE_KEY_V100: &str = "fees_001";
pub const CONFIG_V100: Item<ConfigV100> = Item::new(CONFIG_V100_KEY);
pub const ALLOCATION_HOLDINGSV100: Map<String, AllocationHoldingV100> = Map::new(FEE_KEY_V100);
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct AllocationHoldingV100 {
    pub name: String,            // user-friendly name of wallet
    pub contract: Addr,          // contract/wallet to send too
    pub allocation: u8,          // what portion should we send
    pub send_after: Coin,        // only send $ after we have this amount in this coin
    pub send_type: SendTypeV100, // type of contract/wallet this is
    pub balance: Vec<Coin>,
}
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct ConfigV100 {
    pub this: Addr,
    pub owner: Addr,
    pub gov_contract: Addr,
}
impl ConfigV100 {
    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        if CONFIG_V100_KEY == CONFIG_KEY {
            Err(StdError::generic_err("PFC-Fee-Split: Migration Failed. Config keys are the same"))
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[allow(clippy::upper_case_acronyms)]
pub enum SendTypeV100 {
    WALLET,
    SteakRewards {
        steak: String,
        receiver: String,
    },
}
impl SendTypeV100 {
    pub fn convert(&self, api: &dyn Api, wallet: Addr) -> StdResult<SendType> {
        match self {
            SendTypeV100::WALLET => Ok(SendType::Wallet {
                receiver: wallet,
            }),
            SendTypeV100::SteakRewards {
                steak,
                receiver,
            } => Ok(SendType::SteakRewards {
                steak: api.addr_validate(steak)?,
                receiver: api.addr_validate(receiver)?,
            }),
        }
    }

    pub fn migrate_sendtype_v100(deps: DepsMut) -> StdResult<()> {
        let old_vec = ALLOCATION_HOLDINGSV100
            .range(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        for old in old_vec {
            let new = AllocationHolding {
                name: old.1.name,

                allocation: old.1.allocation,
                send_after: old.1.send_after,
                send_type: old.1.send_type.convert(deps.api, old.1.contract)?,
                balance: old.1.balance,
            };
            ALLOCATION_HOLDINGS.save(deps.storage, old.0, &new)?
        }
        Ok(())
    }
}
