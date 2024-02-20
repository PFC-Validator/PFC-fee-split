use cosmwasm_std::{testing::MOCK_CONTRACT_ADDR, Addr, Uint128};

use crate::mock_querier::custom_deps;

#[test]
fn query_cw20_balance() {
    let mut deps = custom_deps();

    deps.querier.with_token_balances(&[(
        "liquidity0000",
        &[(MOCK_CONTRACT_ADDR, &Uint128::from(123u128))],
    )]);

    assert_eq!(
        Uint128::from(123u128),
        crate::cw20::query_cw20_balance(
            &deps.as_ref().querier,
            &Addr::unchecked("liquidity0000"),
            &Addr::unchecked(MOCK_CONTRACT_ADDR),
        )
        .unwrap()
    );
}
