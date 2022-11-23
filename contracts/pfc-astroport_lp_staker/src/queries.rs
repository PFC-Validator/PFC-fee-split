use cosmwasm_std::{Addr, Deps, Env, Order, StdResult, Storage, Uint128};
use pfc_astroport_lp_staking::errors::ContractError;
use pfc_astroport_lp_staking::lp_staking::query_msgs::{
    ConfigResponse, StakerInfoResponse, StateResponse,
};
use pfc_astroport_lp_staking::lp_staking::TokenBalance;

use crate::states::{Config, StakerInfo, NUM_STAKED, TOTAL_REWARDS, USER_CLAIM};

pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config: Config = Config::load(deps.storage)?;
    let resp = ConfigResponse {
        admin: config.admin.to_string(),
        token: config.token.to_string(),
        pair: config.pair.to_string(),
        lp_token: config.lp_token.to_string(),
        whitelisted_contracts: config
            .whitelisted_contracts
            .iter()
            .map(|item| item.to_string())
            .collect(),
    };

    Ok(resp)
}

pub fn query_state(deps: Deps) -> Result<StateResponse, ContractError> {
    let counters = TOTAL_REWARDS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|f| f.map(|x| x.1))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(StateResponse {
        total_staked: NUM_STAKED.load(deps.storage)?,
        counters_per_token: counters,
    })
}

pub fn query_staker_info(
    deps: Deps,
    _env: Env,
    staker: String,
) -> Result<StakerInfoResponse, ContractError> {
    //let block_height = env.block.height;
    let staker_raw = deps.api.addr_validate(staker.as_str())?;

    let staker_info: StakerInfo = StakerInfo::load_or_default(deps.storage, &staker_raw)?;

    //let config: Config = Config::load(deps.storage)?;
    //let mut state: State = State::load(deps.storage)?;

    //state.compute_reward(&config, block_height);
    //staker_info.compute_staker_reward(&state)?;
    let rewards = calc_token_claims(deps.storage, staker_raw)?;
    Ok(StakerInfoResponse {
        staker,
        total_staked: staker_info.bond_amount,
        estimated_rewards: rewards,
    })
}

pub(crate) fn calc_token_claims(
    storage: &dyn Storage,
    addr: Addr,
) -> Result<Vec<TokenBalance>, ContractError> {
    let mut resp: Vec<TokenBalance> = vec![];
    let staker_info = StakerInfo::load_or_default(storage, &addr)?;
    if staker_info.bond_amount.is_zero() {
        return Ok(vec![]);
    }

    if let Some(user_info) = USER_CLAIM.may_load(storage, addr)? {
        let tallies = TOTAL_REWARDS
            .range(storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        for token in tallies {
            let amt_to_send = if let Some(last_claim) = user_info.get(&token.0) {
                let claim = token.1.amount - last_claim.last_claimed_amount;

                claim * staker_info.bond_amount
            } else {
                Uint128::zero()
            };

            if !amt_to_send.is_zero() {
                resp.push(TokenBalance {
                    token: token.0.to_string(),
                    amount: amt_to_send,
                });
            }
        }
    }

    Ok(resp)
}
