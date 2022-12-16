use crate::entrypoints::{execute, instantiate, query};
use crate::executions::{unbond, withdraw};
use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Attribute, CosmosMsg, Deps, Env, MessageInfo, Response, SubMsg,
    Uint128, WasmMsg,
};
use cw20::Cw20ReceiveMsg;
use pfc_vault::errors::ContractError;
use pfc_vault::mock_querier::CustomDeps;
use pfc_vault::test_constants::liquidity::{
    lp_env, LP_DISTRIBUTION_SCHEDULE1, LP_DISTRIBUTION_SCHEDULE2, LP_LIQUIDITY_TOKEN,
    LP_REWARD_TOKEN,
};
use pfc_vault::test_constants::{default_sender, DEFAULT_SENDER, REWARD_TOKEN};
use pfc_vault::vault::execute_msgs::{Cw20HookMsg, ExecuteMsg, InstantiateMsg};
use pfc_vault::vault::query_msgs::{QueryMsg, StakerInfoResponse};

pub mod bond;
pub mod instantiate;
pub mod unbond;
pub mod update_config;
//pub mod validate;
pub mod merge;
pub mod withdraw;

pub const SENDER_1: &str = "terra1fmcjjt6yc9wqup2r06urnrd928jhrde6gcld6n";
pub const SENDER_2: &str = "terra1333veey879eeqcff8j3gfcgwt8cfrg9mq20v6f";
pub const SENDER_REWARD: &str = "terra14x9fr055x5hvr48hzy2t4q7kvjvfttsvxusa4xsdcy702mnzsvuqprer8r";
pub fn find_attribute<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
    attributes.iter().find(|f| f.key == name)
}

pub fn find_exec(message: &SubMsg) -> Option<&WasmMsg> {
    match &message.msg {
        CosmosMsg::Wasm(wasm) => match wasm {
            WasmMsg::Execute { .. } => Some(wasm),
            _ => None,
        },
        _ => None,
    }
}

pub fn exec_withdraw(
    deps: &mut CustomDeps,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    withdraw(deps.as_mut(), env, info)
}

pub fn exec_bond(
    deps: &mut CustomDeps,
    env: &Env,
    sender: &Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let info = mock_info(LP_LIQUIDITY_TOKEN, &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: sender.to_string(),
        amount,
        msg: to_binary(&Cw20HookMsg::Bond {}).unwrap(),
    });

    execute(deps.as_mut(), env.clone(), info.clone(), msg)
}

pub fn query_staker_info(deps: Deps, env: &Env, sender: &Addr) -> StakerInfoResponse {
    from_binary::<StakerInfoResponse>(
        &query(
            deps,
            env.clone(),
            QueryMsg::StakerInfo {
                staker: sender.to_string(),
            },
        )
        .unwrap(),
    )
    .unwrap()
}
pub fn exec_send_reward_token(
    deps: &mut CustomDeps,
    env: &Env,
    sender: &Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let info = mock_info(REWARD_TOKEN, &[]);
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: sender.to_string(),
        amount,
        msg: Default::default(),
    });

    execute(deps.as_mut(), env.clone(), info.clone(), msg)
}

pub fn exec_unbond(
    deps: &mut CustomDeps,
    env: &Env,
    info: &MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    unbond(deps.as_mut(), env.clone(), info.clone(), amount)
}

pub fn exec_instantiate(
    deps: &mut CustomDeps,
    env: Env,
    info: MessageInfo,
    token: String,
    name: String,
    lp_token: String,
    // whitelisted_contracts: Vec<String>,
) -> Result<Response, ContractError> {
    let msg = InstantiateMsg {
        token,
        name,
        lp_token,
        gov_contract: info.sender.to_string(),
        astroport_generator_contract: None,
    };

    instantiate(deps.as_mut(), env, info, msg)
}

pub fn init_default(
    deps: &mut CustomDeps,
    total_bonded: Option<Uint128>,
) -> (Env, MessageInfo, Response) {
    let env = lp_env();
    let info = default_sender();

    let response = exec_instantiate(
        deps,
        env.clone(),
        info.clone(),
        LP_REWARD_TOKEN.to_string(),
        "Just a name".to_string(),
        LP_LIQUIDITY_TOKEN.to_string(),
    )
    .unwrap();

    if let Some(total_bonded) = total_bonded {
        exec_bond(deps, &env, &default_sender().sender, total_bonded).unwrap();
    }

    deps.querier.plus_token_balances(&[
        (
            LP_REWARD_TOKEN,
            &[(DEFAULT_SENDER, &LP_DISTRIBUTION_SCHEDULE1.2)],
        ),
        (
            LP_REWARD_TOKEN,
            &[(DEFAULT_SENDER, &LP_DISTRIBUTION_SCHEDULE2.2)],
        ),
    ]);

    (env, info, response)
}
