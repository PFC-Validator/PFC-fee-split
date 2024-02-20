use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use pfc_vault::errors::ContractError;

#[cfg(not(feature = "library"))]
use crate::executions::{bond, migrate_reward, unbond, update_config, withdraw};
use crate::{
    queries::{query_config, query_staker_info, query_state},
    states::{Config, ADMIN, NUM_STAKED},
};
/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "pfc-vault";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

use pfc_vault::vault::{
    execute_msgs::{ExecuteMsg, InstantiateMsg, MigrateMsg},
    query_msgs::QueryMsg,
};

use crate::executions::{
    execute_accept_gov_contract, execute_set_new_astroport_generator, execute_update_gov_contract,
    recv_reward_token,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.gov_contract)?;
    let astroport_generator_contract = if let Some(x) = msg.astroport_generator_contract {
        Some(deps.api.addr_validate(&x)?)
    } else {
        None
    };
    ADMIN.set(deps.branch(), Some(admin.clone()))?;

    Config {
        token: deps.api.addr_validate(msg.token.as_str())?,
        name: msg.name.clone(),
        lp_token: deps.api.addr_validate(msg.lp_token.as_str())?,
        gov_contract: admin,
        new_gov_contract: None,
        change_gov_contract_by_height: None,
        astroport_generator_contract,
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
        ExecuteMsg::Unbond {
            amount,
        } => unbond(deps, env, info, amount),
        ExecuteMsg::Withdraw {} => withdraw(deps, env, info),
        ExecuteMsg::UpdateConfig {
            token,
            name,
            // lp_token,
        } => update_config(deps, env, info, token, name /* , lp_token */),
        ExecuteMsg::MigrateReward {
            recipient,
            amount,
        } => migrate_reward(deps, env, info, recipient, amount),

        ExecuteMsg::TransferGovContract {
            gov_contract,
            blocks,
        } => execute_update_gov_contract(deps, env, info, gov_contract, blocks),
        ExecuteMsg::AcceptGovContract {} => execute_accept_gov_contract(deps, env, info),
        ExecuteMsg::SetAstroportGenerator {
            generator,
        } => execute_set_new_astroport_generator(deps, env, info, generator),
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
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::State {} => to_json_binary(&query_state(deps)?),
        QueryMsg::StakerInfo {
            staker,
        } => to_json_binary(&query_staker_info(deps, env, staker)?),
    }?;

    Ok(result)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;

    match contract_version.contract.as_ref() {
        #[allow(clippy::single_match)]
        "pfc-astroport-lp-staker" => {},
        "pfc-vault" => {},
        _ => {
            return Err(ContractError::MigrationError {
                current_name: contract_version.contract,
                current_version: contract_version.version,
            });
        },
    }
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}
