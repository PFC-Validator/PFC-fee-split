use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, SubMsg, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;

use crate::tests::{
    exec_bond, exec_send_reward_token, exec_unbond, exec_withdraw, find_attribute, find_exec,
    init_default, SENDER_1, SENDER_2, SENDER_REWARD,
};
use pfc_astroport_lp_staking::mock_querier::custom_deps;
use pfc_astroport_lp_staking::test_constants::liquidity::{LP_LIQUIDITY_TOKEN, LP_REWARD_TOKEN};

#[test]
fn succeed() {
    let mut deps = custom_deps();
    let sender_reward = Addr::unchecked(SENDER_REWARD);
    let sender1 = mock_info(SENDER_1, &[]);
    let sender2 = mock_info(SENDER_2, &[]);

    let (env, _info, _response) = init_default(&mut deps, None);

    let res = exec_bond(&mut deps, &env, &sender1.sender, Uint128::new(200u128)).unwrap();
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "200");

    let res = exec_bond(&mut deps, &env, &sender2.sender, Uint128::new(200u128)).unwrap();
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "400");

    let res = exec_withdraw(&mut deps, env.clone(), sender1.clone()).unwrap();

    assert_eq!(res.messages.len(), 0);
    let res =
        exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(2000u128)).unwrap();
    let token_attr = find_attribute(&res.attributes, "amount_per_stake").unwrap();
    assert_eq!(token_attr.value, "5");

    assert_eq!(res.messages.len(), 0);

    let res = exec_withdraw(&mut deps, env.clone(), sender1.clone()).unwrap();
    assert_eq!(res.messages.len(), 1);

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: Addr::unchecked(LP_REWARD_TOKEN).to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: Addr::unchecked(SENDER_1).to_string(),
                amount: Uint128::new(1000u128),
                msg: Default::default(),
            })
            .unwrap(),
            funds: vec![],
        }))]
    );
    let res =
        exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(2000u128)).unwrap();
    let token_attr = find_attribute(&res.attributes, "amount_per_stake").unwrap();
    assert_eq!(token_attr.value, "10");
    assert_eq!(res.messages.len(), 0);
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "400");

    //    eprintln!("withdraw SR {:?}", res.attributes);
    let res = exec_unbond(&mut deps, &env, &sender1, Uint128::new(100u128)).unwrap();

    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "300");
    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "100");

    // should return the LP & Reward tokens
    assert_eq!(res.messages.len(), 2);
    let exec = find_exec(&res.messages[0]).unwrap();
    assert_eq!(
        exec,
        &WasmMsg::Execute {
            contract_addr: LP_LIQUIDITY_TOKEN.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: sender1.sender.to_string(),
                amount: Uint128::from(100u64),
                msg: Default::default(),
            })
            .unwrap(),
            funds: vec![],
        }
    );
    let exec = find_exec(&res.messages[1]).unwrap();
    assert_eq!(
        exec,
        &WasmMsg::Execute {
            contract_addr: LP_REWARD_TOKEN.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: sender1.sender.to_string(),
                amount: Uint128::from(1_000u64),
                msg: Default::default(),
            })
            .unwrap(),
            funds: vec![],
        }
    );

    let res =
        exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(2000u128)).unwrap();
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "300");

    eprintln!("withdraw SR2 - {:?}", res.attributes);
    assert_eq!(res.messages.len(), 0);
    let res = exec_withdraw(&mut deps, env.clone(), sender1.clone()).unwrap();
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "300");
    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "100");

    //   eprintln!("withdraw WM {:?}", res.messages);
    assert_eq!(res.messages.len(), 1);
    let exec = find_exec(&res.messages[0]).unwrap();
    assert_eq!(
        exec,
        &WasmMsg::Execute {
            contract_addr: LP_REWARD_TOKEN.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: sender1.sender.to_string(),
                amount: Uint128::from(666u64),
                msg: Default::default(),
            })
            .unwrap(),
            funds: vec![],
        }
    );

    let res = exec_withdraw(&mut deps, env.clone(), sender2.clone()).unwrap();
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "300");
    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "200");
    assert_eq!(res.messages.len(), 1);
    let exec = find_exec(&res.messages[0]).unwrap();
    assert_eq!(
        exec,
        &WasmMsg::Execute {
            contract_addr: LP_REWARD_TOKEN.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: sender2.sender.to_string(),
                amount: Uint128::from(3333u64),
                msg: Default::default(),
            })
            .unwrap(),
            funds: vec![],
        }
    );

    let res = exec_withdraw(&mut deps, env.clone(), sender2.clone()).unwrap();
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "300");
    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "200");
    assert_eq!(res.messages.len(), 0);
}
