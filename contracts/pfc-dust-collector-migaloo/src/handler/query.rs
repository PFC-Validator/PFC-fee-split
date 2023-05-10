use crate::state::{ASSET_HOLDINGS, CONFIG};
use cosmwasm_std::{Addr, Deps, Order, StdResult, Uint128};

use pfc_dust_collector_migaloo::dust_collector::{AssetHolding, CollectorResponse, ConfigResponse};
use std::collections::HashSet;

//const DEFAULT_LIMIT: u32 = 10;
//const MAX_LIMIT: u32 = 30;

pub(crate) fn query_config(deps: Deps) -> StdResult<Option<ConfigResponse>> {
    let config = CONFIG.load(deps.storage)?;

    Ok(Some(ConfigResponse {
        token_router: config.token_router.to_string(),
        base_denom: config.base_denom.to_string(),
        return_contract: config.return_contract.to_string(),
    }))
}

pub(crate) fn query_asset(
    deps: Deps,
    contract_address: &Addr,
    denom: String,
) -> StdResult<Option<AssetHolding>> {
    let minimum = ASSET_HOLDINGS
        .may_load(deps.storage, denom.clone())?
        .unwrap_or(Uint128::zero());
    let coin = deps
        .querier
        .query_balance(contract_address, denom.clone())?;
    Ok(Some(AssetHolding {
        denom,
        minimum,
        balance: coin.amount,
    }))
}

pub(crate) fn query_assets(
    deps: Deps,
    contract_address: &Addr,
) -> StdResult<CollectorResponse<AssetHolding>> {
    let balances = deps.querier.query_all_balances(contract_address)?;
    let mut minimums = ASSET_HOLDINGS
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|f| f.unwrap())
        .collect::<HashSet<String>>();

    let mut holdings: Vec<AssetHolding> = Default::default();
    for coin in balances {
        let minimum = ASSET_HOLDINGS.may_load(deps.storage, coin.denom.clone())?;
        if minimum.is_some() {
            minimums.remove(&coin.denom);
        }
        holdings.push(AssetHolding {
            denom: coin.denom,
            balance: coin.amount,
            minimum: minimum.unwrap_or(Uint128::zero()),
        });
    }
    for denom in minimums {
        let minimum = ASSET_HOLDINGS.may_load(deps.storage, denom.clone())?;
        holdings.push(AssetHolding {
            denom,
            balance: Uint128::zero(),
            minimum: minimum.unwrap_or(Uint128::zero()),
        });
    }

    Ok(CollectorResponse { entries: holdings })
}
