use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{Addr, Env, MessageInfo, Response};
use pfc_astroport_lp_staking::errors::ContractError;

use crate::executions::update_config;
use crate::states::Config;
use pfc_astroport_lp_staking::mock_querier::{custom_deps, CustomDeps};
use pfc_astroport_lp_staking::test_constants::liquidity::*;
use pfc_astroport_lp_staking::test_constants::DEFAULT_SENDER;
use pfc_astroport_lp_staking::test_utils::expect_unauthorized_err;

use crate::tests::{init_default, SENDER_1};

pub fn exec(
    deps: &mut CustomDeps,
    env: Env,
    info: MessageInfo,
    token: Option<String>,
    pair: Option<String>,
    lp_token: Option<String>,
    admin: Option<String>,
    //whitelisted_contracts: Option<Vec<String>>,
) -> Result<Response, ContractError> {
    update_config(
        deps.as_mut(),
        env,
        info,
        token,
        pair,
        lp_token,
        admin,
        //    whitelisted_contracts,
    )
}

pub fn will_success(
    deps: &mut CustomDeps,
    token: Option<String>,
    pair: Option<String>,
    lp_token: Option<String>,
    admin: Option<String>,
    //  whitelisted_contracts: Option<Vec<String>>,
) -> (Env, MessageInfo, Response) {
    let env = lp_env();
    let info = mock_info(DEFAULT_SENDER, &[]);

    let response = exec(
        deps,
        env.clone(),
        info.clone(),
        token,
        pair,
        lp_token,
        admin,
        //   whitelisted_contracts,
    )
    .unwrap();

    (env, info, response)
}

#[test]
fn succeed() {
    let mut deps = custom_deps();

    let (_env, info, _response) = init_default(&mut deps, None);

    will_success(
        &mut deps,
        Some("terra1r0rm0evrlkfvpt0csrcpmnpmrega54czajfd86".to_string()),
        Some(SENDER_1.to_string()),
        Some("terra199vw7724lzkwz6lf2hsx04lrxfkz09tg8dlp6r".to_string()),
        Some("terra1e8ryd9ezefuucd4mje33zdms9m2s90m57878v9".to_string()),
    );

    let config = Config::load(&deps.storage).unwrap();
    assert_eq!(
        config.token,
        Addr::unchecked("terra1r0rm0evrlkfvpt0csrcpmnpmrega54czajfd86".to_string())
    );
    assert_eq!(config.pair, Addr::unchecked(SENDER_1.to_string()));
    assert_eq!(
        config.lp_token,
        Addr::unchecked("terra199vw7724lzkwz6lf2hsx04lrxfkz09tg8dlp6r".to_string())
    );
    assert_eq!(config.admin, info.sender);
    //  assert_eq!(config.whitelisted_contracts, whitelisted_contracts);

    let admin_nominee = Config::may_load_admin_nominee(&deps.storage).unwrap();
    assert_eq!(
        admin_nominee,
        Some(Addr::unchecked(
            "terra1e8ryd9ezefuucd4mje33zdms9m2s90m57878v9".to_string()
        ))
    );
}

#[test]
fn failed_invalid_permission() {
    let mut deps = custom_deps();

    let (env, mut info, _response) = init_default(&mut deps, None);

    info.sender = Addr::unchecked("terra1e8ryd9ezefuucd4mje33zdms9m2s90m57878v9");

    let result = exec(&mut deps, env, info, None, None, None, None);

    expect_unauthorized_err(&result);
}
