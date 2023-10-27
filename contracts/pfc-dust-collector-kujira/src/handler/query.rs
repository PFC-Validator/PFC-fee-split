use crate::state::{ASSET_HOLDINGS, ASSET_STAGES, CONFIG};
use cosmwasm_std::{Addr, Deps, Order, StdResult, Uint128};

use kujira::Denom;
use pfc_dust_collector_kujira::dust_collector::{AssetHolding, CollectorResponse, ConfigResponse};
use std::collections::HashSet;

//const DEFAULT_LIMIT: u32 = 10;
//const MAX_LIMIT: u32 = 30;

pub(crate) fn query_config(deps: Deps) -> StdResult<Option<ConfigResponse>> {
    let config = CONFIG.load(deps.storage)?;

    Ok(Some(ConfigResponse {
        token_router: config.token_router.to_string(),
        base_denom: config.base_denom,
        return_contract: config.return_contract.to_string(),
        max_swaps: config.max_swaps,
    }))
}

pub(crate) fn query_asset(
    deps: Deps,
    contract_address: &Addr,
    denom: Denom,
) -> StdResult<Option<AssetHolding>> {
    let minimum = ASSET_HOLDINGS
        .may_load(deps.storage, denom.to_string())?
        .unwrap_or(Uint128::zero());

    let stages = ASSET_STAGES
        .may_load(deps.storage, denom.to_string())?
        .unwrap_or(Vec::new());
    let coin = deps
        .querier
        .query_balance(contract_address, denom.to_string())?;
    Ok(Some(AssetHolding {
        denom,
        minimum,
        balance: coin.amount,
        stages,
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
        let stages = ASSET_STAGES
            .may_load(deps.storage, coin.denom.clone())?
            .unwrap_or(Vec::new());
        holdings.push(AssetHolding {
            denom: Denom::from(coin.denom),
            balance: coin.amount,
            minimum: minimum.unwrap_or(Uint128::zero()),
            stages,
        });
    }
    for denom in minimums {
        let minimum = ASSET_HOLDINGS.may_load(deps.storage, denom.clone())?;
        let stages = ASSET_STAGES
            .may_load(deps.storage, denom.clone())?
            .unwrap_or(Vec::new());
        holdings.push(AssetHolding {
            denom: Denom::from(denom),
            balance: Uint128::zero(),
            minimum: minimum.unwrap_or(Uint128::zero()),
            stages,
        });
    }

    Ok(CollectorResponse { entries: holdings })
}
