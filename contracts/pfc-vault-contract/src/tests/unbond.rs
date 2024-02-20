use std::ops::Add;

use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Decimal, SubMsg, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use pfc_vault::{
    errors::ContractError, mock_querier::custom_deps, test_constants::REWARD_TOKEN,
    test_utils::expect_generic_err, vault::TokenBalance,
};

use crate::{
    states::StakerInfo,
    tests::{
        exec_send_reward_token, exec_unbond, exec_withdraw, init_default, query_staker_info,
        SENDER_REWARD,
    },
};

#[test]
fn succeed() {
    let mut deps = custom_deps();
    let total_bonded = Uint128::new(200u128);

    let (mut env, info, _response) = init_default(&mut deps, Some(total_bonded));
    env.block.height = 10;
    let _response = exec_unbond(&mut deps, &env, &info, total_bonded).unwrap();

    //    let (_env, info, _response) = will_success(&mut deps, total_bonded);

    let info1 = StakerInfo::load_or_default(deps.as_ref().storage, &info.sender).unwrap();
    assert_eq!(info1.bond_amount, Uint128::zero());
}
#[test]
fn claim_after_unbond() {
    let mut deps = custom_deps();
    let sender_reward = Addr::unchecked(SENDER_REWARD);

    let total_bonded = Uint128::new(200u128);
    let (mut env, info, _response) = init_default(&mut deps, Some(total_bonded));

    let res =
        exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(2_000u128)).unwrap();
    assert_eq!(res.messages.len(), 0);

    let qry = query_staker_info(deps.as_ref(), &env, &info.sender);
    assert_eq!(qry.estimated_rewards.len(), 1);
    assert_eq!(
        qry.estimated_rewards[0],
        TokenBalance {
            amount: Decimal::from_ratio(Uint128::new(2_000u128), Uint128::one()),
            token: Addr::unchecked(REWARD_TOKEN),
            last_block_rewards_seen: 0,
        }
    );
    env.block.height = 10;
    let _response = exec_unbond(&mut deps, &env, &info, total_bonded).unwrap();

    //let (_env, info, _response) = will_success(&mut deps, total_bonded);
    let qry = query_staker_info(deps.as_ref(), &env, &info.sender);
    assert_eq!(qry.estimated_rewards.len(), 1);
    assert_eq!(
        qry.estimated_rewards[0],
        TokenBalance {
            amount: Decimal::new(Uint128::new(2_000u128)),
            token: Addr::unchecked(REWARD_TOKEN),
            last_block_rewards_seen: 0,
        }
    );

    let info1 = StakerInfo::load_or_default(deps.as_ref().storage, &info.sender).unwrap();
    assert_eq!(info1.bond_amount, Uint128::zero());

    let res = exec_withdraw(&mut deps, env.clone(), info.clone()).unwrap();
    assert_eq!(res.messages.len(), 1);

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: Addr::unchecked(REWARD_TOKEN).to_string(),
            /*msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: info.sender.to_string(),
                amount: Uint128::new(2_000u128),
                msg: Default::default(),
            })*/
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: Uint128::from(2_000u128),
            })
            .unwrap(),
            funds: vec![],
        }))]
    );
    let err = exec_withdraw(&mut deps, env.clone(), info.clone()).unwrap_err();
    match err {
        ContractError::NoneBonded {} => {},
        _ => {
            unreachable!("should have failed with NoneBonded")
        },
    }
}

#[test]
fn failed_invalid_amount() {
    let mut deps = custom_deps();
    let total_bonded = Uint128::new(200u128);
    let (env, info, _response) = init_default(&mut deps, Some(total_bonded));
    let result = exec_unbond(&mut deps, &env, &info, total_bonded.add(total_bonded));

    expect_generic_err(&result, "Cannot unbond more than bond amount");
}
