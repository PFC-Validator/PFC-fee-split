use cosmwasm_std::{Addr, Api, Binary, QuerierWrapper, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

//use super::errors::ContractError;
use crate::cw20::query_balance;
use std::cmp::Ordering;
use std::fmt;

//pub type ContractResult<T> = core::result::Result<T, ContractError>;
/*
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OrderBy {
    Asc,
    Desc,
}

impl From<OrderBy> for Order {
    fn from(order_by: OrderBy) -> Self {
        match order_by {
            OrderBy::Asc => Order::Ascending,
            OrderBy::Desc => Order::Descending,
        }
    }
}
*/
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Denom {
    Native(String),
    Token(String),
}

impl Denom {
    pub fn to_cw20(&self, api: &dyn Api) -> cw20::Denom {
        match self {
            Denom::Native(denom) => cw20::Denom::Native(denom.to_string()),
            Denom::Token(contract_addr) => {
                cw20::Denom::Cw20(api.addr_validate(contract_addr).unwrap())
            }
        }
    }

    pub fn from_cw20(denom: cw20::Denom) -> Self {
        match denom {
            cw20::Denom::Native(denom) => Denom::Native(denom),
            cw20::Denom::Cw20(contract_addr) => Denom::Token(contract_addr.to_string()),
        }
    }

    pub fn load_balance(
        &self,
        querier: &QuerierWrapper,
        api: &dyn Api,
        address: Addr,
    ) -> StdResult<Uint128> {
        query_balance(querier, self.to_cw20(api), address)
    }
}

impl fmt::Display for Denom {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Denom::Native(denom) => write!(f, "{}", denom),
            Denom::Token(addr) => write!(f, "{}", addr),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ExecutionMsg {
    pub order: u64,
    pub contract: String,
    pub msg: Binary,
}

impl From<Execution> for ExecutionMsg {
    fn from(e: Execution) -> Self {
        ExecutionMsg {
            order: e.order,
            contract: e.contract.to_string(),
            msg: e.msg,
        }
    }
}

impl From<&Execution> for ExecutionMsg {
    fn from(e: &Execution) -> Self {
        ExecutionMsg {
            order: e.order,
            contract: e.contract.to_string(),
            msg: e.msg.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct Execution {
    pub order: u64,
    pub contract: Addr,
    pub msg: Binary,
}

impl PartialEq for Execution {
    fn eq(&self, other: &Self) -> bool {
        self.order == other.order
    }
}

impl Eq for Execution {}

impl PartialOrd for Execution {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Execution {
    fn cmp(&self, other: &Self) -> Ordering {
        self.order.cmp(&other.order)
    }
}

impl Execution {
    pub fn from(api: &dyn Api, msg: &ExecutionMsg) -> StdResult<Execution> {
        Ok(Execution {
            order: msg.order,
            contract: api.addr_validate(&msg.contract)?,
            msg: msg.msg.clone(),
        })
    }
}


/// We currently take no arguments for migrations
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct EmptyMigrateMsg {}
