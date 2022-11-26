use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};
use cw_controllers::Admin;
use cw_storage_plus::{Item, Map};
use pfc_astroport_lp_staking::lp_staking::TokenBalance;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const CONFIG: Item<Config> = Item::new("config_v1");

pub const ADMIN: Admin = Admin::new("admin");

/// Helper to store number of staked NFTs to increase computational efficiency
pub const NUM_STAKED: Item<Uint128> = Item::new("num_staked_v1");

/// Stores total token rewards PER UNIT NFT since the beginning of time, keyed by CW20 address
pub const TOTAL_REWARDS: Map<Addr, TokenBalance> = Map::new("total_rewards_v1");
pub const USER_LAST_CLAIM: Map<Addr, u64> = Map::new("user_last_claim_v1");

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct UserTokenClaim {
    pub last_claimed_amount: Decimal,
    pub token: Addr,
}

/// Stores total token rewards PER UNIT NFT since the beginning of time, keyed by CW20 address
pub const USER_CLAIM: Map<Addr, Vec<UserTokenClaim>> = Map::new("user_claim_v1");

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    /// The token we send
    pub token: Addr,
    /// The token we 'stake'
    pub lp_token: Addr,
    pub pair: Addr,
    /// 'admin' account
    pub gov_contract: Addr,
    pub new_gov_contract: Option<Addr>,
    pub change_gov_contract_by_height: Option<u64>,
}

impl Config {
    pub fn save(&self, storage: &mut dyn Storage) -> StdResult<()> {
        CONFIG.save(storage, self)
        // Ok(())
    }

    pub fn load(storage: &dyn Storage) -> StdResult<Config> {
        CONFIG.load(storage)
    }
}

const STAKER_INFO: Map<&str, StakerInfo> = Map::new("reward");

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct StakerInfo {
    pub owner: Addr,
    pub bond_amount: Uint128,
}

impl StakerInfo {
    pub fn default(owner: Addr) -> StakerInfo {
        StakerInfo {
            owner,
            bond_amount: Uint128::zero(),
        }
    }

    pub fn save(&self, storage: &mut dyn Storage) -> StdResult<()> {
        STAKER_INFO.save(storage, self.owner.as_str(), self)
    }

    pub fn load_or_default(storage: &dyn Storage, owner: &Addr) -> StdResult<StakerInfo> {
        Ok(STAKER_INFO
            .may_load(storage, owner.as_str())?
            .unwrap_or_else(|| StakerInfo::default(owner.clone())))
    }

    pub fn delete(&self, storage: &mut dyn Storage) {
        STAKER_INFO.remove(storage, self.owner.as_str())
    }
}
