use std::collections::HashMap;

use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult, Storage, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use pfc_vault::errors::ContractError;
use pfc_vault::message_factories;
use pfc_vault::vault::TokenBalance;

use crate::states::{
    Config, PendingClaimAmount, StakerInfo, UserTokenClaim, ADMIN, NUM_STAKED, TOTAL_REWARDS,
    USER_CLAIM, USER_LAST_CLAIM, USER_PENDING_CLAIM,
};
use crate::utils::merge_claims;

pub fn bond(
    deps: DepsMut,
    env: Env,
    sender_addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let sender_addr_raw: Addr = deps.api.addr_validate(sender_addr.as_str())?;

    let mut staker_info: StakerInfo = StakerInfo::load_or_default(deps.storage, &sender_addr_raw)?;

    if staker_info.bond_amount.is_zero() {
        let tallies = TOTAL_REWARDS
            .range(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        USER_CLAIM.save(
            deps.storage,
            sender_addr_raw.clone(),
            &tallies
                .iter()
                .map(|tb| UserTokenClaim {
                    last_claimed_amount: tb.1.amount,
                    token: tb.0.clone(),
                })
                .collect::<Vec<UserTokenClaim>>(),
        )?;
        //    } else {
    }
    //  let msgs = do_token_claims_and_gen_messages(deps.storage, env.block.height, &sender_addr_raw)?;
    update_token_claims(deps.storage, env.block.height, &sender_addr_raw)?;

    // Increase bond_amount
    let num_staked = NUM_STAKED.update(deps.storage, |num| -> StdResult<Uint128> {
        Ok(num + amount)
    })?;
    staker_info.bond_amount += amount;
    staker_info.save(deps.storage)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "bond"),
        ("owner", &sender_addr),
        ("amount_bonded", &amount.to_string()),
        ("amount_staked", &staker_info.bond_amount.to_string()),
        // ("amount_per_stake", &amount_per_stake.to_string()),
        ("total_staked", &num_staked.to_string()),
    ]))
    // .add_messages(msgs))
}

///
/// unbond - sends the remaining rewards, decrements the user's staked, &  total staked
pub fn unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config: Config = Config::load(deps.storage)?;
    let sender_addr_raw: Addr = info.sender;

    let mut staker_info: StakerInfo = StakerInfo::load_or_default(deps.storage, &sender_addr_raw)?;

    if staker_info.bond_amount < amount {
        return Err(ContractError::Std(StdError::generic_err(
            "Cannot unbond more than bond amount",
        )));
    }

    //  let msgs = do_token_claims_and_gen_messages(deps.storage, env.block.height, &sender_addr_raw)?;
    update_token_claims(deps.storage, env.block.height, &sender_addr_raw)?;

    // Decrease bond_amount
    let num_staked = NUM_STAKED.update(deps.storage, |num| -> StdResult<Uint128> {
        Ok(num.checked_sub(amount)?)
    })?;

    staker_info.bond_amount = (staker_info.bond_amount.checked_sub(amount))?;
    if staker_info.bond_amount.is_zero() {
        //no bond, remove.
        staker_info.delete(deps.storage);
    } else {
        staker_info.save(deps.storage)?;
    }

    Ok(Response::new()
        .add_message(message_factories::wasm_execute(
            &config.lp_token,
            &Cw20ExecuteMsg::Transfer {
                recipient: sender_addr_raw.to_string(),
                amount,
                //msg: Default::default(),
            },
        ))
        // .add_messages(msgs)
        .add_attribute("action", "unbond")
        .add_attribute("owner", sender_addr_raw.to_string())
        .add_attribute("amount", amount.to_string())
        .add_attribute("amount_staked", staker_info.bond_amount.to_string())
        .add_attribute("total_staked", num_staked.to_string()))
}

pub fn recv_reward_token(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // Calculate amount to distribute
    let num_staked = NUM_STAKED.load(deps.storage)?;
    //   eprintln!("Num_staked ={} msg.amount={}", num_staked, msg.amount);

    if num_staked.is_zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "num staked is zero",
        )));
    }
    let amount_per_stake = Decimal::from_ratio(msg.amount, 1u128)
        .checked_div(Decimal::from_ratio(num_staked, 1u128))?;

    if amount_per_stake.is_zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "Amount per stake is zero",
        )));
    }
    let upd_token =
        if let Some(mut token) = TOTAL_REWARDS.may_load(deps.storage, info.sender.clone())? {
            token.amount += amount_per_stake;
            token.last_block_rewards_seen = env.block.height;
            token
        } else {
            TokenBalance {
                amount: amount_per_stake,
                token: info.sender.clone(),
                last_block_rewards_seen: env.block.height,
            }
        };
    TOTAL_REWARDS.save(deps.storage, info.sender.clone(), &upd_token)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "recv_reward_token"),
        ("token_addr", info.sender.as_str()),
        ("token_sender", &msg.sender),
        ("total_amount", &msg.amount.to_string()),
        ("amount_per_stake", &upd_token.amount.to_string()),
        ("total_staked", &num_staked.to_string()),
    ]))
}

// withdraw rewards to executor
pub fn withdraw(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let sender_addr_raw = info.sender;

    let staker_info = StakerInfo::load_or_default(deps.storage, &sender_addr_raw)?;
    let has_pending = USER_PENDING_CLAIM
        .may_load(deps.storage, sender_addr_raw.clone())?
        .unwrap_or_default();

    if staker_info.bond_amount.is_zero() && has_pending.is_empty() {
        staker_info.delete(deps.storage);
        Err(ContractError::NoneBonded {})
    } else {
        let num_staked = NUM_STAKED.load(deps.storage)?;
        let msgs =
            do_token_claims_and_gen_messages(deps.storage, env.block.height, &sender_addr_raw)?;

        Ok(Response::new()
            .add_attributes(vec![
                ("action", "withdraw"),
                ("amount_staked", &staker_info.bond_amount.to_string()),
                ("total_staked", &num_staked.to_string()),
            ])
            .add_messages(msgs))
    }
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token: Option<String>,
    name: Option<String>,
    //   lp_token: Option<String>,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_config");

    let mut config: Config = Config::load(deps.storage)?;

    if let Some(token) = token {
        if let Some(reward) = TOTAL_REWARDS.may_load(deps.storage, config.token)? {
            if !reward.amount.is_zero() {
                return Err(ContractError::RewardsPresent {});
            }
        }
        config.token = deps.api.addr_validate(token.as_str())?;
        response = response.add_attribute("is_updated_token", "true");
    }

    if let Some(name) = name {
        config.name = name;
        response = response.add_attribute("is_updated_name", "true");
    }
    /*
        if let Some(lp_token) = lp_token {
            config.lp_token = deps.api.addr_validate(lp_token.as_str())?;
            response = response.add_attribute("is_updated_lp_token", "true");
        }
    */
    config.save(deps.storage)?;

    Ok(response)
}

pub fn migrate_reward(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = Config::load(deps.storage)?;
    if let Some(astro) = config.astroport_generator_contract {
        if info.sender != astro {
            ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
        }
    } else {
        ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    }

    Ok(Response::new()
        .add_attribute("action", "migrate_reward")
        .add_message(message_factories::wasm_execute(
            &config.token,
            &Cw20ExecuteMsg::Transfer {
                recipient: (deps.api.addr_validate(recipient.as_str())?).to_string(),
                amount,
            },
        ))
        .add_attribute("recipient", recipient)
        .add_attribute("amount", amount.to_string()))
}

pub fn execute_update_gov_contract(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    gov_contract: String,
    blocks: u64,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let new_admin = deps.api.addr_validate(&gov_contract)?;
    let mut config = Config::load(deps.storage)?;

    config.new_gov_contract = Some(new_admin);
    config.change_gov_contract_by_height = Some(env.block.height + blocks);
    config.save(deps.storage)?;

    let res = Response::new().add_attribute("action", "update_gov_contract");
    Ok(res)
}

pub fn execute_set_new_astroport_generator(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    generator_contract: Option<String>,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let astroport_generator_contract = if let Some(x) = generator_contract {
        Some(deps.api.addr_validate(&x)?)
    } else {
        None
    };
    let mut config = Config::load(deps.storage)?;

    config.new_gov_contract = astroport_generator_contract;

    config.save(deps.storage)?;

    let res = Response::new().add_attribute("action", "set_new_astroport_generator");
    Ok(res)
}

pub fn execute_accept_gov_contract(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = Config::load(deps.storage)?;

    if let Some(new_admin) = config.new_gov_contract {
        if new_admin != info.sender {
            Err(ContractError::Unauthorized {})
        } else if let Some(block_height) = config.change_gov_contract_by_height {
            if block_height < env.block.height {
                Err(ContractError::Unauthorized {})
            } else {
                config.gov_contract = new_admin.clone();
                config.new_gov_contract = None;
                config.change_gov_contract_by_height = None;
                config.save(deps.storage)?;
                ADMIN.set(deps, Some(new_admin))?;
                let res = Response::new().add_attribute("action", "accept_gov_contract");
                Ok(res)
            }
        } else {
            Err(ContractError::Unauthorized {})
        }
    } else {
        Err(ContractError::Unauthorized {})
    }
}

/// calculates the amount of claims outstanding, storing it in USER_PENDING_CLAIM but not sending it
pub(crate) fn update_token_claims(
    storage: &mut dyn Storage,
    block_height: u64,
    addr: &Addr,
) -> Result<(), ContractError> {
    let previous = USER_PENDING_CLAIM
        .may_load(storage, addr.clone())?
        .unwrap_or_default();
    let current = get_current_claims(storage, block_height, addr)?;
    let merged = merge_claims(&previous, &current);
    USER_PENDING_CLAIM.save(storage, addr.clone(), &merged)?;
    Ok(())
}

pub(crate) fn get_current_claims(
    storage: &mut dyn Storage,
    block_height: u64,
    addr: &Addr,
) -> Result<Vec<PendingClaimAmount>, ContractError> {
    let mut resp: Vec<PendingClaimAmount> = vec![];
    let mut new_claims: Vec<UserTokenClaim> = vec![];

    let tallies = TOTAL_REWARDS
        .range(storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    let staker_info = StakerInfo::load_or_default(storage, addr)?;
    if staker_info.bond_amount.is_zero() {
        return Ok(vec![]);
    }

    USER_LAST_CLAIM.save(storage, addr.clone(), &block_height)?;

    let user_info_vec = USER_CLAIM
        .may_load(storage, addr.clone())?
        .unwrap_or_default();
    let user_info = user_info_vec
        .iter()
        .map(|ui| (ui.token.clone(), ui))
        .collect::<HashMap<Addr, &UserTokenClaim>>();

    for token in tallies {
        let amt = if let Some(last_claim) = user_info.get(&token.0) {
            token.1.amount - last_claim.last_claimed_amount
        } else {
            token.1.amount
        };

        let amt_to_send = staker_info.bond_amount * amt;
        new_claims.push(UserTokenClaim {
            last_claimed_amount: token.1.amount,
            token: token.1.token,
        });

        if !amt_to_send.is_zero() {
            resp.push(PendingClaimAmount {
                token: token.0,
                amount: amt_to_send,
            });
        }
    }

    USER_CLAIM.save(storage, addr.clone(), &new_claims)?;
    Ok(resp)
}

pub(crate) fn gen_claim_messages(
    storage: &mut dyn Storage,
    addr: &Addr,
    clear_pending: bool,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let mut resp: Vec<CosmosMsg> = vec![];
    if let Some(pending) = USER_PENDING_CLAIM.may_load(storage, addr.clone())? {
        for claim_amount in pending {
            if !claim_amount.amount.is_zero() {
                let msg = Cw20ExecuteMsg::Transfer {
                    recipient: addr.to_string(),
                    amount: claim_amount.amount,
                };

                resp.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: claim_amount.token.to_string(),
                    msg: to_json_binary(&msg)?,
                    funds: vec![],
                }))
            }
        }
        if clear_pending {
            USER_PENDING_CLAIM.save(storage, addr.clone(), &vec![])?
        }
    }
    Ok(resp)
}

pub(crate) fn do_token_claims_and_gen_messages(
    storage: &mut dyn Storage,
    block_height: u64,
    addr: &Addr,
) -> Result<Vec<CosmosMsg>, ContractError> {
    update_token_claims(storage, block_height, addr)?;
    let resp = gen_claim_messages(storage, addr, true)?;

    Ok(resp)
}
