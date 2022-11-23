use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult,
    Storage, Uint128, WasmMsg,
};

use crate::states::{Config, StakerInfo, UserTokenClaim, NUM_STAKED, TOTAL_REWARDS, USER_CLAIM};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use pfc_astroport_lp_staking::errors::ContractError;
use pfc_astroport_lp_staking::lp_staking::TokenBalance;
use pfc_astroport_lp_staking::message_factories;
use pfc_astroport_lp_staking::utils::is_contract;

pub fn bond(
    deps: DepsMut,
    _env: Env,
    sender_addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let sender_addr_raw: Addr = deps.api.addr_validate(sender_addr.as_str())?;

    let config: Config = Config::load(deps.storage)?;

    // TBD - do we only want wallets?
    if !config.is_authorized(&sender_addr_raw)? {
        return Err(ContractError::Std(StdError::generic_err(
            "Can only called by wallet",
        )));
    }

    let msgs = do_token_claims(deps.storage, &sender_addr_raw)?;

    let mut staker_info: StakerInfo = StakerInfo::load_or_default(deps.storage, &sender_addr_raw)?;

    // Increase bond_amount
    let num_staked = NUM_STAKED.update(deps.storage, |num| -> StdResult<Uint128> {
        Ok(num + amount)
    })?;
    staker_info.bond_amount += amount;
    staker_info.save(deps.storage)?;

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "bond"),
            ("owner", &sender_addr),
            ("amount_bonded", &amount.to_string()),
            ("amount_staked", &staker_info.bond_amount.to_string()),
            // ("amount_per_stake", &amount_per_stake.to_string()),
            ("total_staked", &num_staked.to_string()),
        ])
        .add_messages(msgs))
}

///
/// unbond - sends the remaining rewards, decrements the user's staked, &  total staked
pub fn unbond(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config: Config = Config::load(deps.storage)?;
    let sender_addr_raw: Addr = info.sender;

    //   let mut state: State = State::load(deps.storage)?;
    let mut staker_info: StakerInfo = StakerInfo::load_or_default(deps.storage, &sender_addr_raw)?;

    if staker_info.bond_amount < amount {
        return Err(ContractError::Std(StdError::generic_err(
            "Cannot unbond more than bond amount",
        )));
    }

    let msgs = do_token_claims(deps.storage, &sender_addr_raw)?;

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
            },
        ))
        .add_messages(msgs)
        .add_attribute("owner", sender_addr_raw.to_string())
        .add_attribute("amount", amount.to_string())
        .add_attribute("amount_staked", &staker_info.bond_amount.to_string())
        .add_attribute("total_staked", num_staked.to_string()))
}

pub fn recv_reward_token(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // Calculate amount to distribute
    let num_staked = NUM_STAKED.load(deps.storage)?;
    let amount_per_stake = msg.amount.checked_div(num_staked)?;

    if amount_per_stake.is_zero() {
        return Err(ContractError::Std(StdError::generic_err(
            "Amount per stake is zero",
        )));
    }
    let upd_token =
        if let Some(mut token) = TOTAL_REWARDS.may_load(deps.storage, info.sender.clone())? {
            token.amount += amount_per_stake;
            token
        } else {
            TokenBalance {
                amount: amount_per_stake,
                token: String::from(info.sender.clone()),
            }
        };
    TOTAL_REWARDS.save(deps.storage, info.sender.clone(), &upd_token)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "recv_reward_token"),
        ("token_addr", info.sender.as_str()),
        ("token_sender", &msg.sender),
        ("total_amount", &msg.amount.to_string()),
        ("amount_per_stake", &amount_per_stake.to_string()),
        ("num_staked", &num_staked.to_string()),
    ]))
}
// withdraw rewards to executor
pub fn withdraw(deps: DepsMut, _env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let sender_addr_raw = info.sender;

    let staker_info = StakerInfo::load_or_default(deps.storage, &sender_addr_raw)?;

    if staker_info.bond_amount.is_zero() {
        staker_info.delete(deps.storage);
        Err(ContractError::NoneBonded {})
    } else {
        let msgs = do_token_claims(deps.storage, &sender_addr_raw)?;

        Ok(Response::new()
            .add_attributes(vec![
                ("action", "withdraw"),
                ("amount_staked", &staker_info.bond_amount.to_string()),
            ])
            .add_messages(msgs))
    }
}

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token: Option<String>,
    pair: Option<String>,
    lp_token: Option<String>,
    admin: Option<String>,
    whitelisted_contracts: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    let mut response = Response::new().add_attribute("action", "update_config");

    let mut config: Config = Config::load(deps.storage)?;
    if config.admin != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(token) = token {
        config.token = deps.api.addr_validate(token.as_str())?;
        response = response.add_attribute("is_updated_token", "true");
    }

    if let Some(pair) = pair {
        config.pair = deps.api.addr_validate(pair.as_str())?;
        response = response.add_attribute("is_updated_pair", "true");
    }

    if let Some(lp_token) = lp_token {
        config.lp_token = deps.api.addr_validate(lp_token.as_str())?;
        response = response.add_attribute("is_updated_lp_token", "true");
    }

    if let Some(admin) = admin {
        Config::save_admin_nominee(deps.storage, &deps.api.addr_validate(admin.as_str())?)?;
        response = response.add_attribute("is_updated_admin_nominee", "true");
    }

    if let Some(whitelisted_contracts) = whitelisted_contracts {
        config.whitelisted_contracts = whitelisted_contracts
            .iter()
            .map(|item| deps.api.addr_validate(item.as_str()).unwrap())
            .collect();
        response = response.add_attribute("is_updated_whitelisted_contracts", "true");
    }

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
    if config.admin != info.sender {
        return Err(ContractError::Unauthorized {});
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

pub fn approve_admin_nominee(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Execute
    if let Some(admin_nominee) = Config::may_load_admin_nominee(deps.storage)? {
        if admin_nominee != info.sender {
            return Err(ContractError::Std(StdError::generic_err(
                "It is not admin nominee",
            )));
        }
    } else {
        return Err(ContractError::Unauthorized {});
    }

    let mut config = Config::load(deps.storage)?;
    config.admin = info.sender;

    config.save(deps.storage)?;

    Ok(Response::new()
        .add_attribute("action", "approve_admin_nominee")
        .add_attribute("is_updated_admin", "true"))
}

pub(crate) fn do_token_claims(
    storage: &mut dyn Storage,
    addr: &Addr,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let mut resp: Vec<CosmosMsg> = vec![];
    let staker_info = StakerInfo::load_or_default(storage, addr)?;
    if staker_info.bond_amount.is_zero() {
        return Ok(vec![]);
    }

    if let Some(mut user_info) = USER_CLAIM.may_load(storage, addr.clone())? {
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
            user_info.insert(
                token.0.clone(),
                UserTokenClaim {
                    last_claimed_amount: token.1.amount,
                    token: token.1.token,
                },
            );

            if !amt_to_send.is_zero() {
                let msg = if is_contract(addr) {
                    Cw20ExecuteMsg::Send {
                        contract: addr.to_string(),
                        amount: amt_to_send,
                        msg: Default::default(),
                    }
                } else {
                    Cw20ExecuteMsg::Transfer {
                        recipient: addr.to_string(),
                        amount: amt_to_send,
                    }
                };
                resp.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: token.0.to_string(),
                    msg: to_binary(&msg)?,
                    funds: vec![],
                }))
            }
        }
        USER_CLAIM.save(storage, addr.clone(), &user_info)?
    }

    Ok(resp)
}