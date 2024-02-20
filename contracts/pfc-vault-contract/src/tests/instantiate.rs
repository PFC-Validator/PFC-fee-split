use pfc_vault::{mock_querier::custom_deps, test_constants::liquidity::*};

use crate::{states::Config, tests::init_default};

#[test]
fn succeed() {
    let mut deps = custom_deps();

    let (_env, info, _response) = init_default(&mut deps, None);

    let config = Config::load(&deps.storage).unwrap();
    assert_eq!(config.token, LP_REWARD_TOKEN);
    assert_eq!(config.name, "Just a name");
    assert_eq!(config.lp_token, LP_LIQUIDITY_TOKEN);
    assert_eq!(config.gov_contract, info.sender);
}
