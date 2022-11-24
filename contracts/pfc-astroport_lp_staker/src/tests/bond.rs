use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{to_binary, Addr, Decimal, Env, Response, Uint128, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use pfc_astroport_lp_staking::errors::ContractError;
use pfc_astroport_lp_staking::lp_staking::execute_msgs::{Cw20HookMsg, ExecuteMsg};
use pfc_astroport_lp_staking::mock_querier::{custom_deps, CustomDeps};
use pfc_astroport_lp_staking::test_constants::liquidity::LP_LIQUIDITY_TOKEN;
use pfc_astroport_lp_staking::test_constants::REWARD_TOKEN;

use crate::entrypoints::execute;
use crate::executions::withdraw;
use crate::states::{StakerInfo, NUM_STAKED, USER_CLAIM};
use crate::tests::instantiate::default;
use crate::tests::{find_attribute, find_exec};

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

fn will_success(deps: &mut CustomDeps, env: Env, sender: &Addr) {
    let amount = Uint128::new(100u128);
    exec_bond(deps, &env, sender, amount).unwrap();
}

#[test]
fn succeed() {
    let sender1 = Addr::unchecked("terra1fmcjjt6yc9wqup2r06urnrd928jhrde6gcld6n");
    let sender2 = Addr::unchecked("terra1333veey879eeqcff8j3gfcgwt8cfrg9mq20v6f");
    let sender3 =
        Addr::unchecked("terra14x9fr055x5hvr48hzy2t4q7kvjvfttsvxusa4xsdcy702mnzsvuqprer8r");

    let mut deps = custom_deps();
    let (env, _info, _response) = default(&mut deps, None);
    will_success(&mut deps, env.clone(), &sender1);
    will_success(&mut deps, env.clone(), &sender2);

    //    let state1 = State::load(deps.as_ref().storage).unwrap();
    let info1 = StakerInfo::load_or_default(deps.as_ref().storage, &sender1).unwrap();
    let info2 = StakerInfo::load_or_default(deps.as_ref().storage, &sender2).unwrap();
    let num_staked = NUM_STAKED.load(deps.as_ref().storage).unwrap();

    assert_eq!(num_staked, Uint128::new(200u128));
    assert_eq!(info1.bond_amount, Uint128::new(100u128));
    assert_eq!(info2.bond_amount, Uint128::new(100u128));

    let res =
        exec_send_reward_token(&mut deps, &env, &sender3, Uint128::new(1_000_000u128)).unwrap();

    let token_attr = find_attribute(&res.attributes, "token_addr").unwrap();
    assert_eq!(token_attr.value, REWARD_TOKEN);

    let token_attr = find_attribute(&res.attributes, "amount_per_stake").unwrap();
    assert_eq!(token_attr.value, "5000");

    let token_attr = find_attribute(&res.attributes, "total_amount").unwrap();
    assert_eq!(token_attr.value, "1000000");

    let token_attr = find_attribute(&res.attributes, "num_staked").unwrap();
    assert_eq!(token_attr.value, "200");

    let info1 = StakerInfo::load_or_default(deps.as_ref().storage, &sender1).unwrap();
    let info2 = StakerInfo::load_or_default(deps.as_ref().storage, &sender2).unwrap();
    let res = withdraw(deps.as_mut(), env, mock_info(sender1.as_str(), &[])).unwrap();
    assert_eq!(res.messages.len(), 1);
    let exec = find_exec(&res.messages[0]).unwrap();
    assert_eq!(
        exec,
        &WasmMsg::Execute {
            contract_addr: REWARD_TOKEN.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: sender1.to_string(),
                amount: Uint128::from(500_000u64),
                msg: Default::default(),
            })
            .unwrap(),
            funds: vec![],
        }
    );

    let token_attr = find_attribute(&res.attributes, "amount_staked").unwrap();
    assert_eq!(token_attr.value, "100");

    let token_claims1 = USER_CLAIM.load(deps.as_ref().storage, sender1).unwrap();
    let token_claim1 = token_claims1
        .into_iter()
        .find(|p| p.token == REWARD_TOKEN)
        .unwrap();

    let num_staked = NUM_STAKED.load(deps.as_ref().storage).unwrap();

    assert_eq!(num_staked, Uint128::new(200u128));
    assert_eq!(info1.bond_amount, Uint128::new(100u128));
    assert_eq!(info2.bond_amount, Uint128::new(100u128));
    assert_eq!(
        token_claim1.last_claimed_amount,
        Decimal::new(Uint128::from(1_000_000u128))
            .checked_div(Decimal::new(Uint128::from(200u128)))
            .unwrap()
    );
}
