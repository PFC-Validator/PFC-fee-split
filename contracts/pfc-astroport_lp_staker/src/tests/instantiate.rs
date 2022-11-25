use crate::states::Config;
use crate::tests::init_default;
use pfc_astroport_lp_staking::mock_querier::custom_deps;
use pfc_astroport_lp_staking::test_constants::liquidity::*;

#[test]
fn succeed() {
    let mut deps = custom_deps();

    let (_env, info, _response) = init_default(&mut deps, None);

    let config = Config::load(&deps.storage).unwrap();
    assert_eq!(config.token, LP_REWARD_TOKEN);
    assert_eq!(config.pair, LP_PAIR_TOKEN);
    assert_eq!(config.lp_token, LP_LIQUIDITY_TOKEN);
    assert_eq!(config.admin, info.sender);
    /*
    assert_eq!(
        config.whitelisted_contracts,
        vec![LP_WHITELISTED1.to_string(), LP_WHITELISTED2.to_string()]
    );

        assert_eq!(
            config.distribution_schedule,
            vec![LP_DISTRIBUTION_SCHEDULE1, LP_DISTRIBUTION_SCHEDULE2]
        );

        let state = State::load(&deps.storage).unwrap();
        assert_eq!(state.global_reward_index, Decimal::zero());
        assert_eq!(state.last_distributed, env.block.height);
        assert_eq!(state.total_bond_amount, Uint128::zero());
    */
}
