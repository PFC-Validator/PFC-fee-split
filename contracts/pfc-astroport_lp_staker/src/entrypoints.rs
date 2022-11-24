#[cfg(not(feature = "library"))]
use crate::executions::{bond, migrate_reward, unbond, update_config, withdraw};
use crate::queries::{query_config, query_staker_info, query_state};
use crate::states::{Config, NUM_STAKED};
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, Uint128};
use cw20::Cw20ReceiveMsg;
use pfc_astroport_lp_staking::errors::ContractError;

use crate::executions::recv_reward_token;
use pfc_astroport_lp_staking::lp_staking::execute_msgs::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use pfc_astroport_lp_staking::lp_staking::query_msgs::QueryMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Config {
        admin: info.sender,
        token: deps.api.addr_validate(msg.token.as_str())?,

        pair: deps.api.addr_validate(msg.pair.as_str())?,
        lp_token: deps.api.addr_validate(msg.lp_token.as_str())?,
        /*
        whitelisted_contracts: msg
            .whitelisted_contracts
            .iter()
            .map(|item| deps.api.addr_validate(item.as_str()).unwrap())
            .collect(),*/
    }
    .save(deps.storage)?;

    NUM_STAKED.save(deps.storage, &Uint128::zero())?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::Unbond { amount } => unbond(deps, env, info, amount),
        ExecuteMsg::Withdraw {} => withdraw(deps, env, info),
        ExecuteMsg::UpdateConfig {
            token,
            pair,
            lp_token,
            admin,
            //   whitelisted_contracts,
            //    distribution_schedule,
        } => update_config(
            deps, env, info, token, pair, lp_token,
            admin,
            //    whitelisted_contracts,
            //   distribution_schedule,
        ),
        ExecuteMsg::MigrateReward { recipient, amount } => {
            migrate_reward(deps, env, info, recipient, amount)
        }
        ExecuteMsg::ApproveAdminNominee {} => {
            crate::executions::approve_admin_nominee(deps, env, info)
        }
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config: Config = Config::load(deps.storage)?;
    let sender = deps.api.addr_validate(info.sender.as_str())?;

    if config.lp_token == sender {
        //
        bond(deps, env, cw20_msg.sender, cw20_msg.amount)
    } else if config.token == sender {
        // this isn't really necessary. we can accept multiple CW20s

        recv_reward_token(deps, env, info, cw20_msg)
        //    }
    } else {
        Err(ContractError::Unauthorized {})
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let result = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::StakerInfo { staker } => to_binary(&query_staker_info(deps, env, staker)?),
    }?;

    Ok(result)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}
