#[cfg(not(feature = "library"))]
use crate::executions::{bond, migrate_reward, unbond, update_config, withdraw};
use crate::queries::{query_config, query_staker_info, query_state};
use crate::states::{Config, ADMIN, NUM_STAKED};
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, Uint128};
use cw20::Cw20ReceiveMsg;
use pfc_astroport_lp_staking::errors::ContractError;

use crate::executions::{
    execute_accept_gov_contract, execute_update_gov_contract, recv_reward_token,
};
use pfc_astroport_lp_staking::lp_staking::execute_msgs::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use pfc_astroport_lp_staking::lp_staking::query_msgs::QueryMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let admin = deps.api.addr_validate(&msg.gov_contract)?;
    ADMIN.set(deps.branch(), Some(admin.clone()))?;

    Config {
        token: deps.api.addr_validate(msg.token.as_str())?,
        name: msg.name.clone(),
        lp_token: deps.api.addr_validate(msg.lp_token.as_str())?,
        gov_contract: admin,
        new_gov_contract: None,
        change_gov_contract_by_height: None,
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
            name,
            lp_token,
        } => update_config(deps, env, info, token, name, lp_token),
        ExecuteMsg::MigrateReward { recipient, amount } => {
            migrate_reward(deps, env, info, recipient, amount)
        }

        ExecuteMsg::TransferGovContract {
            gov_contract,
            blocks,
        } => execute_update_gov_contract(deps, env, info, gov_contract, blocks),
        ExecuteMsg::AcceptGovContract {} => execute_accept_gov_contract(deps, env, info),
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
