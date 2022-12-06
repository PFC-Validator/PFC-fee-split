use crate::states::StakerInfo;
use cosmwasm_std::{Env, MessageInfo, Response, Uint128};

use crate::tests::{exec_unbond, init_default};
use pfc_vault::mock_querier::{custom_deps, CustomDeps};
use pfc_vault::test_utils::expect_generic_err;
use std::ops::Add;

pub fn will_success(deps: &mut CustomDeps, amount: Uint128) -> (Env, MessageInfo, Response) {
    let (mut env, info, _response) = init_default(deps, Some(amount));

    env.block.height = 10;

    let response = exec_unbond(deps, &env, &info, amount).unwrap();
    (env, info, response)
}

#[test]
fn succeed() {
    let mut deps = custom_deps();
    let total_bonded = Uint128::new(200u128);
    let (_env, info, _response) = will_success(&mut deps, total_bonded);

    let info1 = StakerInfo::load_or_default(deps.as_ref().storage, &info.sender).unwrap();
    assert_eq!(info1.bond_amount, Uint128::zero());
}

#[test]
fn failed_invalid_amount() {
    let mut deps = custom_deps();
    let total_bonded = Uint128::new(200u128);
    let (env, info, _response) = init_default(&mut deps, Some(total_bonded));
    let result = exec_unbond(&mut deps, &env, &info, total_bonded.add(total_bonded));

    expect_generic_err(&result, "Cannot unbond more than bond amount");
}
