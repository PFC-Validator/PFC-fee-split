use cosmwasm_std::{testing::mock_info, to_json_binary, Addr, Decimal, Env, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use pfc_vault::{
    mock_querier::{custom_deps, CustomDeps},
    test_constants::REWARD_TOKEN,
};

use crate::{
    executions::withdraw,
    states::{NUM_STAKED, USER_CLAIM},
    tests::{
        exec_bond, exec_send_reward_token, find_attribute, find_exec, init_default,
        query_staker_info, SENDER_1, SENDER_2, SENDER_REWARD,
    },
};

fn will_success(deps: &mut CustomDeps, env: Env, sender: &Addr) {
    let amount = Uint128::new(100u128);
    exec_bond(deps, &env, sender, amount).unwrap();
}

#[test]
fn succeed() {
    let sender1 = Addr::unchecked(SENDER_1);
    let sender2 = Addr::unchecked(SENDER_2);
    let sender_reward = Addr::unchecked(SENDER_REWARD);

    let mut deps = custom_deps();
    let (env, _info, _response) = init_default(&mut deps, None);
    will_success(&mut deps, env.clone(), &sender1);
    will_success(&mut deps, env.clone(), &sender2);

    //    let state1 = State::load(deps.as_ref().storage).unwrap();
    let info1 = query_staker_info(deps.as_ref(), &env, &sender1);
    let info2 = query_staker_info(deps.as_ref(), &env, &sender2);

    let num_staked = NUM_STAKED.load(deps.as_ref().storage).unwrap();

    assert_eq!(num_staked, Uint128::new(200u128));
    assert_eq!(info1.total_staked, Uint128::new(100u128));
    assert_eq!(info2.total_staked, Uint128::new(100u128));

    let res = exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(1_000_000u128))
        .unwrap();

    let token_attr = find_attribute(&res.attributes, "token_addr").unwrap();
    assert_eq!(token_attr.value, REWARD_TOKEN);

    let token_attr = find_attribute(&res.attributes, "amount_per_stake").unwrap();
    assert_eq!(token_attr.value, "5000");

    let token_attr = find_attribute(&res.attributes, "total_amount").unwrap();
    assert_eq!(token_attr.value, "1000000");

    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "200");

    //    let info1 = StakerInfo::load_or_default(deps.as_ref().storage, &sender1).unwrap();
    //    let info2 = StakerInfo::load_or_default(deps.as_ref().storage, &sender2).unwrap();
    let info1 = query_staker_info(deps.as_ref(), &env, &sender1);
    let info2 = query_staker_info(deps.as_ref(), &env, &sender2);

    let res = withdraw(deps.as_mut(), env, mock_info(sender1.as_str(), &[])).unwrap();
    assert_eq!(res.messages.len(), 1);
    let exec = find_exec(&res.messages[0]).unwrap();
    assert_eq!(
        exec,
        &WasmMsg::Execute {
            contract_addr: REWARD_TOKEN.to_string(),
            /*
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: sender1.to_string(),
                amount: Uint128::from(500_000u64),
                msg: Default::default(),
            })*/
            msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender1.to_string(),
                amount: Uint128::from(500_000u64),
            })
            .unwrap(),
            funds: vec![],
        }
    );

    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "100");

    let token_claims1 = USER_CLAIM.load(deps.as_ref().storage, sender1).unwrap();
    let token_claim1 = token_claims1.into_iter().find(|p| p.token == REWARD_TOKEN).unwrap();

    let num_staked = NUM_STAKED.load(deps.as_ref().storage).unwrap();

    assert_eq!(num_staked, Uint128::new(200u128));
    assert_eq!(info1.total_staked, Uint128::new(100u128));
    assert_eq!(info2.total_staked, Uint128::new(100u128));
    assert_eq!(
        token_claim1.last_claimed_amount,
        Decimal::new(Uint128::from(1_000_000u128))
            .checked_div(Decimal::new(Uint128::from(200u128)))
            .unwrap()
    );
}

#[test]
fn none_bonded() {
    let sender_reward = Addr::unchecked(SENDER_REWARD);

    let mut deps = custom_deps();
    let (env, _info, _response) = init_default(&mut deps, None);

    let num_staked = NUM_STAKED.load(deps.as_ref().storage).unwrap();

    assert_eq!(num_staked, Uint128::new(0u128));

    let _err = exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(1_000_000u128))
        .unwrap_err();
}
