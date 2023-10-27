// TBD - this is temporary until the package is public
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin};
use kujira::Denom;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub fee: u128,
    pub treasury: String,
    pub blend_oracle_contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Swap {
        stages: Vec<Vec<(Addr, Denom)>>,
        recipient: Option<Addr>,
        min_return: Option<Vec<Coin>>,
    },
    UpdateConfig {
        fee: Option<u128>,
        owner: Option<String>,
        treasury: Option<String>,
        blend_oracle_contract: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(UserResponse)]
    UserScore { address: String, week: u128 },
    #[returns(SwapsResponse)]
    TotalSwaps { week: u128 },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub fee: u128,
    pub treasury: Addr,
    pub blend_oracle_contract: Addr,
}

#[cw_serde]
pub struct UserResponse {
    pub address: String,
    pub week: u128,
    pub value: u128,
}

#[cw_serde]
pub struct SwapsResponse {
    pub week: u128,
    pub value: u128,
}

#[cw_serde]
pub struct BlendOracleQuery {
    pub price: BlendCoinWrapper,
}

#[cw_serde]
pub struct BlendCoinWrapper {
    pub coin: BlendCoin,
}

#[cw_serde]
pub struct BlendCoin {
    pub denom: String,
    pub amount: String,
}

#[cw_serde]
pub struct BlendOracleResponse {
    pub price: BlendOracleDenom,
}

#[cw_serde]
pub struct BlendOracleDenom {
    pub denom: String,
    pub amount: String,
}
