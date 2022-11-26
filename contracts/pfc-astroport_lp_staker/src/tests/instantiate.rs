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
    assert_eq!(config.gov_contract, info.sender);
}
