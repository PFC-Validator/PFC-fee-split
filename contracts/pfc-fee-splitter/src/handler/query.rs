use crate::state::{ALLOCATION_HOLDINGS, CONFIG, FLUSH_WHITELIST};
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use pfc_fee_split::fee_split_msg::{
    AllocationHolding, AllocationResponse, OwnershipResponse, WhitelistResponse,
};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

pub(crate) fn query_gov_contract(deps: Deps) -> StdResult<OwnershipResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(OwnershipResponse {
        owner: config.gov_contract.to_string(),
        new_owner: config.new_gov_contract.map(|x| x.to_string()),
        block_height: config.change_gov_contract_by_height,
    })
}

pub(crate) fn query_allocation(deps: Deps, name: String) -> StdResult<Option<AllocationHolding>> {
    ALLOCATION_HOLDINGS.may_load(deps.storage, name)
}

pub(crate) fn query_flush_whitelist(deps: Deps) -> StdResult<WhitelistResponse> {
    let list = FLUSH_WHITELIST
        .items(deps.storage, None, None, Order::Ascending)
        .map(|x| x.map(|f| f.to_string()))
        .collect::<StdResult<Vec<String>>>()?;
    Ok(WhitelistResponse { allowed: list })
}

pub(crate) fn query_allocations(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllocationResponse> {
    let limit_amt = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);
    Ok(AllocationResponse {
        allocations: ALLOCATION_HOLDINGS
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit_amt)
            .map(|item| item.map(|(_, v)| v))
            .collect::<StdResult<Vec<AllocationHolding>>>()?,
    })
}
