use crate::entrypoints::query;
use crate::tests::bond::exec_bond;
use crate::tests::instantiate::default;
use crate::tests::unbond::exec_unbond;
use cosmwasm_std::{from_binary, Env, Uint128};
use pfc_astroport_lp_staking::lp_staking::query_msgs::{QueryMsg, StakerInfoResponse};
use pfc_astroport_lp_staking::mock_querier::{custom_deps, CustomDeps};

fn query_staker_info(deps: &CustomDeps, env: Env, sender: String) -> StakerInfoResponse {
    from_binary::<StakerInfoResponse>(
        &query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::StakerInfo { staker: sender },
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn calculation() {
    let mut deps = custom_deps();

    let (mut env, info, _response) = default(&mut deps, None);

    // bond 100 tokens
    exec_bond(&mut deps, &env, &info.sender, Uint128::new(100u128)).unwrap();
    env.block.height += 100;

    exec_bond(&mut deps, &env, &info.sender, Uint128::new(100u128)).unwrap();

    let res = query_staker_info(&deps, env.clone(), info.sender.to_string());
    //  assert_eq!(res.pending_reward, Uint128::new(1000000u128));
    //assert_eq!(res.bond_amount, Uint128::new(200u128));

    env.block.height += 10;
    exec_unbond(&mut deps, env.clone(), info.clone(), Uint128::new(100u128)).unwrap();

    let res = query_staker_info(&deps, env.clone(), info.sender.to_string());
    //assert_eq!(res.pending_reward, Uint128::new(2000000u128));
    //assert_eq!(res.bond_amount, Uint128::new(100u128));

    env.block.height += 10;

    let res = query_staker_info(&deps, env.clone(), info.sender.to_string());
    //assert_eq!(res.pending_reward, Uint128::new(3000000u128));
    //assert_eq!(res.bond_amount, Uint128::new(100u128));
}