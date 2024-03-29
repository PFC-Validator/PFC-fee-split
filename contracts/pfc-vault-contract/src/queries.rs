use std::{collections::HashMap, ops::Add};

use cosmwasm_std::{Addr, Decimal, Deps, Env, Order, StdResult, Storage};
use pfc_vault::{
    errors::ContractError,
    vault::{
        query_msgs::{ConfigResponse, StakerInfoResponse, StateResponse},
        TokenBalance,
    },
};

use crate::states::{
    Config, PendingClaimAmount, StakerInfo, UserTokenClaim, NUM_STAKED, TOTAL_REWARDS, USER_CLAIM,
    USER_LAST_CLAIM, USER_PENDING_CLAIM,
};

pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config: Config = Config::load(deps.storage)?;
    let resp = ConfigResponse {
        token: config.token.to_string(),
        name: config.name.to_string(),
        lp_token: config.lp_token.to_string(),
        gov_contract: config.gov_contract.to_string(),
        new_gov_contract: config.new_gov_contract.map(|f| f.to_string()),
        change_gov_contract_by_height: config.change_gov_contract_by_height,
        astroport_generator_contract: config.astroport_generator_contract.map(|f| f.to_string()),
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

    let pending_claim =
        USER_PENDING_CLAIM.may_load(deps.storage, staker_raw.clone())?.unwrap_or_default();

    let rewards_vec = calc_token_claims(deps.storage, &staker_raw)?;
    let pending = pending_claim
        .into_iter()
        .map(|f| {
            (
                f.token.clone(),
                PendingClaimAmount {
                    amount: f.amount,
                    token: f.token,
                },
            )
        })
        .collect::<HashMap<Addr, PendingClaimAmount>>();
    let mut rewards = rewards_vec
        .into_iter()
        .map(|f| {
            (
                f.token.clone(),
                TokenBalance {
                    amount: f.amount,
                    token: f.token.clone(),
                    last_block_rewards_seen: f.last_block_rewards_seen,
                },
            )
        })
        .collect::<HashMap<Addr, TokenBalance>>();
    for pending_tb in pending {
        rewards
            .entry(pending_tb.0)
            .and_modify(|e| e.amount = e.amount.add(Decimal::new(pending_tb.1.amount)))
            .or_insert(TokenBalance {
                amount: Decimal::new(pending_tb.1.amount),
                token: pending_tb.1.token.clone(),
                last_block_rewards_seen: 0,
            });
    }
    let last_claim = USER_LAST_CLAIM.may_load(deps.storage, staker_raw)?;

    Ok(StakerInfoResponse {
        staker,
        total_staked: staker_info.bond_amount,
        estimated_rewards: rewards.into_iter().map(|f| f.1).collect(),
        last_claimed: last_claim,
    })
}

pub(crate) fn calc_token_claims(
    storage: &dyn Storage,
    addr: &Addr,
) -> Result<Vec<TokenBalance>, ContractError> {
    let mut resp: Vec<TokenBalance> = vec![];
    let staker_info = StakerInfo::load_or_default(storage, addr)?;
    if staker_info.bond_amount.is_zero() {
        return Ok(vec![]);
    }

    let user_info_vec = USER_CLAIM.may_load(storage, addr.clone())?.unwrap_or_default();
    let user_info = user_info_vec
        .iter()
        .map(|ui| (ui.token.clone(), ui))
        .collect::<HashMap<Addr, &UserTokenClaim>>();

    let tallies = TOTAL_REWARDS
        .range(storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    for token in tallies {
        let amt = if let Some(last_claim) = user_info.get(&token.0) {
            token.1.amount - last_claim.last_claimed_amount
        } else {
            token.1.amount
        };
        let amt_to_send = amt.checked_mul(Decimal::from_ratio(staker_info.bond_amount, 1u128))?;

        if !amt_to_send.is_zero() {
            resp.push(TokenBalance {
                token: token.0,
                amount: amt_to_send,
                last_block_rewards_seen: token.1.last_block_rewards_seen,
            });
        }
    }

    Ok(resp)
}
