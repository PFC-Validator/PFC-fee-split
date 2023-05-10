use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{Addr, Env, MessageInfo, Response};

use pfc_vault::errors::ContractError;
use pfc_vault::mock_querier::{custom_deps, CustomDeps};
use pfc_vault::test_constants::liquidity::*;
use pfc_vault::test_constants::DEFAULT_SENDER;
use pfc_vault::test_utils::expect_unauthorized_err;

use crate::executions::{execute_accept_gov_contract, execute_update_gov_contract, update_config};
use crate::states::Config;
use crate::tests::{init_default, SENDER_1};

pub fn exec(
    deps: &mut CustomDeps,
    env: Env,
    info: MessageInfo,
    token: Option<String>,
    pair: Option<String>,
    lp_token: Option<String>,
) -> Result<Response, ContractError> {
    update_config(deps.as_mut(), env, info, token, pair, lp_token)
}

pub fn will_success(
    deps: &mut CustomDeps,
    token: Option<String>,
    name: Option<String>,
    lp_token: Option<String>,
) -> (Env, MessageInfo, Response) {
    let env = lp_env();
    let info = mock_info(DEFAULT_SENDER, &[]);

    let response = exec(deps, env.clone(), info.clone(), token, name, lp_token).unwrap();

    (env, info, response)
}

#[test]
fn succeed() {
    let mut deps = custom_deps();

    let (_env, info, _response) = init_default(&mut deps, None);

    will_success(
        &mut deps,
        Some("terra1r0rm0evrlkfvpt0csrcpmnpmrega54czajfd86".to_string()),
        Some("NEW NAME".to_string()),
        Some("terra199vw7724lzkwz6lf2hsx04lrxfkz09tg8dlp6r".to_string()),
        //  Some("terra1e8ryd9ezefuucd4mje33zdms9m2s90m57878v9".to_string()),
    );

    let config = Config::load(&deps.storage).unwrap();
    assert_eq!(
        config.token,
        Addr::unchecked("terra1r0rm0evrlkfvpt0csrcpmnpmrega54czajfd86".to_string())
    );
    assert_eq!(config.name, "NEW NAME".to_string());
    assert_eq!(
        config.lp_token,
        Addr::unchecked("terra199vw7724lzkwz6lf2hsx04lrxfkz09tg8dlp6r".to_string())
    );
    assert_eq!(config.gov_contract, info.sender);
}

#[test]
fn switch_gov_contract() {
    let mut deps = custom_deps();

    let (env, info, _response) = init_default(&mut deps, None);
    let sender1 = MessageInfo {
        sender: Addr::unchecked(SENDER_1),
        funds: vec![],
    };
    let new_admin = MessageInfo {
        sender: Addr::unchecked("terra1e8ryd9ezefuucd4mje33zdms9m2s90m57878v9"),
        funds: vec![],
    };
    let _res = execute_update_gov_contract(
        deps.as_mut(),
        env.clone(),
        sender1.clone(),
        new_admin.sender.to_string(),
        100,
    )
    .unwrap_err();
    let _res = execute_update_gov_contract(
        deps.as_mut(),
        env.clone(),
        info,
        new_admin.sender.to_string(),
        100,
    )
    .unwrap();
    let config = Config::load(&deps.storage).unwrap();
    assert_eq!(
        config.change_gov_contract_by_height.unwrap(),
        env.block.height + 100
    );
    assert_eq!(config.new_gov_contract.unwrap(), new_admin.sender,);
    let _res = execute_accept_gov_contract(deps.as_mut(), env.clone(), sender1).unwrap_err();
    let _res = execute_accept_gov_contract(deps.as_mut(), env.clone(), new_admin).unwrap();
}

#[test]
fn failed_invalid_permission() {
    let mut deps = custom_deps();

    let (env, mut info, _response) = init_default(&mut deps, None);

    info.sender = Addr::unchecked("terra1e8ryd9ezefuucd4mje33zdms9m2s90m57878v9");

    let result = exec(&mut deps, env, info, None, None, None);

    expect_unauthorized_err(&result);
}
