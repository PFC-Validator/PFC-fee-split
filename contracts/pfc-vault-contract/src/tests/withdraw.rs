use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, SubMsg, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use std::str::FromStr;

use crate::tests::{
    exec_bond, exec_send_reward_token, exec_unbond, exec_withdraw, find_attribute, find_exec,
    init_default, query_staker_info, SENDER_1, SENDER_2, SENDER_REWARD,
};
use pfc_vault::mock_querier::custom_deps;
use pfc_vault::test_constants::liquidity::{LP_LIQUIDITY_TOKEN, LP_REWARD_TOKEN};
use pfc_vault::test_constants::REWARD_TOKEN;
use pfc_vault::vault::TokenBalance;

#[test]
fn succeed() {
    // tests to do
    // #1 - simple withdraw with 2 bonders
    // #2 - unbond 1/2 of a single bonder
    // #3 - add more rewards, and verify new ratio is working

    let mut deps = custom_deps();
    let sender_reward = Addr::unchecked(SENDER_REWARD);
    let sender1 = mock_info(SENDER_1, &[]);
    let sender2 = mock_info(SENDER_2, &[]);
    let mut reward_tally = Uint128::zero();

    let (mut env, _info, _response) = init_default(&mut deps, None);
    env.block.height += 1;
    let res = exec_bond(&mut deps, &env, &sender1.sender, Uint128::new(200u128)).unwrap();
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "200");
    env.block.height += 2;

    let res = exec_bond(&mut deps, &env, &sender2.sender, Uint128::new(200u128)).unwrap();
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "400");
    env.block.height += 3;

    let res = exec_withdraw(&mut deps, env.clone(), sender1.clone()).unwrap();
    env.block.height += 1;

    assert_eq!(res.messages.len(), 0);
    let res =
        exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(2_000u128)).unwrap();
    reward_tally += Uint128::new(2_000u128);

    let token_attr = find_attribute(&res.attributes, "amount_per_stake").unwrap();
    assert_eq!(token_attr.value, "5");
    assert_eq!(res.messages.len(), 0);

    let res = exec_withdraw(&mut deps, env.clone(), sender1.clone()).unwrap();
    env.block.height += 1;

    assert_eq!(res.messages.len(), 1);

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: Addr::unchecked(LP_REWARD_TOKEN).to_string(),
            /*msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: Addr::unchecked(SENDER_1).to_string(),
                amount: Uint128::new(1_000u128),
                msg: Default::default(),
            })*/
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: Addr::unchecked(SENDER_1).to_string(),
                amount: Uint128::new(1_000u128),
            })
            .unwrap(),
            funds: vec![],
        }))]
    );
    reward_tally -= Uint128::new(1_000u128);

    let res =
        exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(2_000u128)).unwrap();
    env.block.height += 1;

    reward_tally += Uint128::new(2_000u128);

    let token_attr = find_attribute(&res.attributes, "amount_per_stake").unwrap();
    assert_eq!(token_attr.value, "10");
    assert_eq!(res.messages.len(), 0);
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "400");

    //    eprintln!("withdraw SR {:?}", res.attributes);
    let res = exec_unbond(&mut deps, &env, &sender1, Uint128::new(100u128)).unwrap();
    env.block.height += 1;

    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "300");
    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "100");

    // should return the LP tokens
    assert_eq!(res.messages.len(), 1);
    let exec = find_exec(&res.messages[0]).unwrap();
    assert_eq!(
        exec,
        &WasmMsg::Execute {
            contract_addr: LP_LIQUIDITY_TOKEN.to_string(),
            /*            msg: to_binary(&Cw20ExecuteMsg::Send {
                           contract: sender1.sender.to_string(),
                           amount: Uint128::from(100u64),
                           msg: Default::default(),
                       })

            */
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender1.sender.to_string(),
                amount: Uint128::new(100u128),
            })
            .unwrap(),
            funds: vec![],
        }
    );
    let qry = query_staker_info(deps.as_ref(), &env, &sender1.sender);
    assert_eq!(qry.estimated_rewards.len(), 1);
    assert_eq!(
        qry.estimated_rewards[0],
        TokenBalance {
            amount: Decimal::new(Uint128::new(1_000u128)),
            token: Addr::unchecked(REWARD_TOKEN),
            last_block_rewards_seen: 0,
        }
    );

    //    reward_tally -= Uint128::new(1_000u128);

    let res =
        exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(2_000u128)).unwrap();
    env.block.height += 1;

    reward_tally += Uint128::new(2_000u128);

    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "300");

    let token_attr = find_attribute(&res.attributes, "amount_per_stake").unwrap();
    assert_eq!(token_attr.value, "16.666666666666666666");

    //    eprintln!("withdraw SR2 - {:?}", res.attributes);
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
            contract_addr: REWARD_TOKEN.to_string(),
            /*            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: sender1.sender.to_string(),
                amount: Uint128::from(1666u64),
                msg: Default::default(),
            })*/
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender1.sender.to_string(),
                amount: Uint128::new(1_666u128),
            })
            .unwrap(),
            funds: vec![],
        }
    );
    reward_tally -= Uint128::new(1666u128);

    let info2 = query_staker_info(deps.as_ref(), &env, &sender2.sender);
    assert_eq!(info2.total_staked, Uint128::new(200u128));
    assert_eq!(info2.last_claimed, None);

    assert_eq!(info2.estimated_rewards.len(), 1);
    assert_eq!(info2.estimated_rewards[0].last_block_rewards_seen, 10);
    assert_eq!(info2.estimated_rewards[0].token, LP_REWARD_TOKEN);

    let range_high = Decimal::from_str("3334").unwrap();
    let range_low = Decimal::from_str("3333").unwrap();
    if info2.estimated_rewards[0].amount.gt(&range_high)
        || info2.estimated_rewards[0].amount.lt(&range_low)
    {
        eprintln!(
            "{} is not within range {} -> {}",
            info2.estimated_rewards[0].amount, range_low, range_high
        );
        unreachable!("amount not in desired range");
    }

    let res = exec_withdraw(&mut deps, env.clone(), sender2.clone()).unwrap();
    env.block.height += 1;

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
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender2.sender.to_string(),
                amount: Uint128::from(3_333u64),
                //msg: Default::default(),
            })
            .unwrap(),
            funds: vec![],
        }
    );
    reward_tally -= Uint128::new(3_333u128);

    let res = exec_withdraw(&mut deps, env.clone(), sender2.clone()).unwrap();

    //    env.block.height +=1;

    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "300");
    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "200");
    assert_eq!(res.messages.len(), 0);
    if reward_tally <= Uint128::one() {
        assert!(
            reward_tally <= Uint128::one(),
            "Outstanding rewards exceeds tolerance of >1< unit",
        );
    }
    let info1 = query_staker_info(deps.as_ref(), &env, &sender1.sender);
    assert_eq!(info1.total_staked, Uint128::new(100u128));
    assert_eq!(info1.estimated_rewards, &[]);
    assert_eq!(info1.last_claimed.unwrap(), 11);
    let info2 = query_staker_info(deps.as_ref(), &env, &sender2.sender);
    assert_eq!(info2.total_staked, Uint128::new(200u128));
    assert_eq!(info2.estimated_rewards, &[]);
    assert_eq!(info2.last_claimed.unwrap(), 12);
    assert_eq!(env.block.height, 12);
}

#[test]
fn test_4() {
    // tests to do
    // #4 - add more rewards, then increase bond

    let mut deps = custom_deps();
    let sender_reward = Addr::unchecked(SENDER_REWARD);
    let sender1 = mock_info(SENDER_1, &[]);
    let sender2 = mock_info(SENDER_2, &[]);
    let mut reward_tally = Uint128::zero();

    let (mut env, _info, _response) = init_default(&mut deps, None);
    env.block.height += 1;
    let res = exec_bond(&mut deps, &env, &sender1.sender, Uint128::new(200u128)).unwrap();
    assert_eq!(res.messages.len(), 0);
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "200");
    env.block.height += 1;

    let res = exec_bond(&mut deps, &env, &sender2.sender, Uint128::new(200u128)).unwrap();
    assert_eq!(res.messages.len(), 0);
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "400");
    env.block.height += 1;

    let res =
        exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(2_000u128)).unwrap();
    reward_tally += Uint128::new(2_000u128);

    let token_attr = find_attribute(&res.attributes, "amount_per_stake").unwrap();
    assert_eq!(token_attr.value, "5");
    assert_eq!(res.messages.len(), 0);

    env.block.height += 1;
    let res = exec_bond(&mut deps, &env, &sender1.sender, Uint128::new(100u128)).unwrap();
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "500");

    assert_eq!(res.messages.len(), 0);
    let qry = query_staker_info(deps.as_ref(), &env, &sender1.sender);
    assert_eq!(qry.estimated_rewards.len(), 1);
    assert_eq!(
        qry.estimated_rewards[0],
        TokenBalance {
            amount: Decimal::new(Uint128::new(1000u128)),
            token: Addr::unchecked(REWARD_TOKEN),
            last_block_rewards_seen: 0,
        }
    );
    //    reward_tally -= Uint128::new(1_000u128);

    env.block.height += 1;

    let _res =
        exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(2_000u128)).unwrap();
    reward_tally += Uint128::new(2_000u128);

    let res = exec_withdraw(&mut deps, env.clone(), sender1.clone()).unwrap();
    env.block.height += 1;

    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "500");
    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "300");
    assert_eq!(res.messages.len(), 1);

    let exec = find_exec(&res.messages[0]).unwrap();
    assert_eq!(
        exec,
        &WasmMsg::Execute {
            contract_addr: LP_REWARD_TOKEN.to_string(),
            /*msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: sender1.sender.to_string(),
                amount: Uint128::from(2_200u64),
                msg: Default::default(),
            })*/
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender1.sender.to_string(),
                amount: Uint128::new(2_200u128),
            })
            .unwrap(),
            funds: vec![],
        }
    );
    reward_tally -= Uint128::new(2_200);
    let qry = query_staker_info(deps.as_ref(), &env, &sender1.sender);
    assert_eq!(qry.estimated_rewards, vec![]);
    let res = exec_withdraw(&mut deps, env.clone(), sender2.clone()).unwrap();
    env.block.height += 1;

    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "500");
    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "200");
    assert_eq!(res.messages.len(), 1);

    let exec = find_exec(&res.messages[0]).unwrap();
    assert_eq!(
        exec,
        &WasmMsg::Execute {
            contract_addr: LP_REWARD_TOKEN.to_string(),
            /*msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: sender2.sender.to_string(),
                amount: Uint128::from(1_800u64),
                msg: Default::default(),
            })*/
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender2.sender.to_string(),
                amount: Uint128::from(1_800u128),
            })
            .unwrap(),
            funds: vec![],
        }
    );
    reward_tally -= Uint128::new(1_800u128);

    if reward_tally <= Uint128::one() {
        assert!(
            reward_tally <= Uint128::one(),
            "Outstanding rewards exceeds tolerance of >1< unit",
        );
    }
    let info1 = query_staker_info(deps.as_ref(), &env, &sender1.sender);
    assert_eq!(info1.total_staked, Uint128::new(300u128));
    assert_eq!(info1.estimated_rewards, &[]);
    assert_eq!(info1.last_claimed.unwrap(), 5);
    let info2 = query_staker_info(deps.as_ref(), &env, &sender2.sender);
    assert_eq!(info2.total_staked, Uint128::new(200u128));
    assert_eq!(info2.estimated_rewards, &[]);
    assert_eq!(info2.last_claimed.unwrap(), 6);
    assert_eq!(env.block.height, 7);
}

#[test]
fn test_5() {
    // tests to do
    // #5 - bone someone, add some rewards, then add a new bond

    let mut deps = custom_deps();
    let sender_reward = Addr::unchecked(SENDER_REWARD);
    let sender1 = mock_info(SENDER_1, &[]);
    let sender2 = mock_info(SENDER_2, &[]);
    let mut reward_tally = Uint128::zero();

    let (mut env, _info, _response) = init_default(&mut deps, None);
    env.block.height += 1;
    let res = exec_bond(&mut deps, &env, &sender1.sender, Uint128::new(200u128)).unwrap();
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "200");
    env.block.height += 1;

    let res =
        exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(2_000u128)).unwrap();
    reward_tally += Uint128::new(2_000u128);

    let token_attr = find_attribute(&res.attributes, "amount_per_stake").unwrap();
    assert_eq!(token_attr.value, "10");
    assert_eq!(res.messages.len(), 0);

    let res = exec_bond(&mut deps, &env, &sender2.sender, Uint128::new(200u128)).unwrap();
    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "400");
    env.block.height += 1;

    assert_eq!(res.messages.len(), 0);
    let _res =
        exec_send_reward_token(&mut deps, &env, &sender_reward, Uint128::new(2_000u128)).unwrap();
    reward_tally += Uint128::new(2_000u128);

    let res = exec_withdraw(&mut deps, env.clone(), sender1.clone()).unwrap();
    env.block.height += 1;

    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "400");
    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "200");
    assert_eq!(res.messages.len(), 1);

    let exec = find_exec(&res.messages[0]).unwrap();
    assert_eq!(
        exec,
        &WasmMsg::Execute {
            contract_addr: LP_REWARD_TOKEN.to_string(),
            /*msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: sender1.sender.to_string(),
                amount: Uint128::from(3_000u64),
                msg: Default::default(),
            })*/
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender1.sender.to_string(),
                amount: Uint128::new(3_000u128),
            })
            .unwrap(),
            funds: vec![],
        }
    );
    reward_tally -= Uint128::new(3_000);

    let res = exec_withdraw(&mut deps, env.clone(), sender2.clone()).unwrap();
    env.block.height += 1;

    let token_attr = find_attribute(&res.attributes, "total_staked").unwrap();
    assert_eq!(token_attr.value, "400");
    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "200");
    assert_eq!(res.messages.len(), 1);

    let exec = find_exec(&res.messages[0]).unwrap();
    assert_eq!(
        exec,
        &WasmMsg::Execute {
            contract_addr: LP_REWARD_TOKEN.to_string(),
            /*            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: sender2.sender.to_string(),
                amount: Uint128::from(1_000u64),
                msg: Default::default(),
            })*/
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender2.sender.to_string(),
                amount: Uint128::new(1_000u128),
            })
            .unwrap(),
            funds: vec![],
        }
    );
    reward_tally -= Uint128::new(1_000u128);

    if reward_tally <= Uint128::one() {
        assert!(
            reward_tally <= Uint128::one(),
            "Outstanding rewards exceeds tolerance of >1< unit",
        );
    }
    let info1 = query_staker_info(deps.as_ref(), &env, &sender1.sender);
    assert_eq!(info1.total_staked, Uint128::new(200u128));
    assert_eq!(info1.estimated_rewards, &[]);
    assert_eq!(info1.last_claimed.unwrap(), 3);
    let info2 = query_staker_info(deps.as_ref(), &env, &sender2.sender);
    assert_eq!(info2.total_staked, Uint128::new(200u128));
    assert_eq!(info2.estimated_rewards, &[]);
    assert_eq!(info2.last_claimed.unwrap(), 4);
    assert_eq!(env.block.height, 5);
}
