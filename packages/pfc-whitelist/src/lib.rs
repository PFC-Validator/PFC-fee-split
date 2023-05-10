use std::collections::HashSet;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Order, StdError, StdResult, Storage};
use cw_storage_plus::{Bound, Map};

pub const DEFAULT_LIMIT: u32 = 10;
pub const MAX_LIMIT: u32 = 30;

#[cfg(test)]
mod tests;

#[cw_serde]
pub struct Whitelist {
    pub address: String,
    pub reason: Option<String>,
}
/// Errors associated with the contract's ownership
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum WhitelistError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("List Supplied to Whitelist is not unique")]
    NotUnique(),
    #[error("Whitelist entry {name} already exists")]
    EntryExists { name: String },
    #[error("Whitelist entry {name} does not exist")]
    EntryDoesntExist { name: String },
}
const WHITELIST: Map<String, Option<String>> = Map::new("whitelist");

#[cw_serde]
pub struct WhitelistResponse<T> {
    pub entries: Vec<T>,
}

pub fn initialize_whitelist(
    storage: &mut dyn Storage,
    api: &dyn Api,
    addresses: Vec<Whitelist>,
) -> Result<(), WhitelistError> {
    let dupe_check: HashSet<String> = addresses.iter().map(|v| v.address.clone()).collect();
    if dupe_check.len() != addresses.len() {
        return Err(WhitelistError::NotUnique {});
    }
    for rec in addresses {
        let addr = api.addr_validate(&rec.address)?;
        WHITELIST.save(storage, addr.to_string(), &rec.reason)?;
    }
    Ok(())
}
pub fn add_entry(
    storage: &mut dyn Storage,
    api: &dyn Api,
    address: String,
    reason: Option<String>,
) -> Result<(), WhitelistError> {
    let addr = api.addr_validate(&address)?;
    let entry_exists = WHITELIST.may_load(storage, address.clone())?;
    if entry_exists.is_some() {
        return Err(WhitelistError::EntryExists { name: address });
    }

    WHITELIST.save(storage, addr.to_string(), &reason)?;

    Ok(())
}
pub fn remove_entry(
    storage: &mut dyn Storage,
    api: &dyn Api,
    name: String,
) -> Result<(), WhitelistError> {
    let addr = api.addr_validate(&name)?;
    let entry_exists = WHITELIST.may_load(storage, addr.to_string())?;
    if let Some(_entry) = entry_exists {
        WHITELIST.remove(storage, addr.to_string());
        Ok(())
    } else {
        Err(WhitelistError::EntryDoesntExist { name })
    }
}
pub fn query_entry(storage: &dyn Storage, address: String) -> StdResult<Option<Whitelist>> {
    if let Some(reason) = WHITELIST.may_load(storage, address.clone())? {
        Ok(Some(Whitelist { address, reason }))
    } else {
        Ok(None)
    }
}

pub fn query_entries(
    storage: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<WhitelistResponse<Whitelist>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    //    let start_addr = maybe_addr(deps.api, start_after)?;
    let start = start_after.as_ref().map(Bound::exclusive);
    let res = WHITELIST
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|x| {
            x.map(|y| Whitelist {
                address: y.0,
                reason: y.1,
            })
        })
        .collect::<StdResult<Vec<Whitelist>>>()?;
    Ok(WhitelistResponse { entries: res })
}

pub fn is_listed(storage: &dyn Storage, address: &Addr) -> StdResult<Option<Whitelist>> {
    if let Some(entry) = WHITELIST.may_load(storage, address.to_string())? {
        Ok(Some(Whitelist {
            address: address.to_string(),
            reason: entry,
        }))
    } else {
        Ok(None)
    }
}
pub fn assert_listed(storage: &dyn Storage, address: &Addr) -> Result<(), WhitelistError> {
    if WHITELIST.may_load(storage, address.to_string())?.is_some() {
        Ok(())
    } else {
        Err(WhitelistError::EntryDoesntExist {
            name: address.to_string(),
        })
    }
}
