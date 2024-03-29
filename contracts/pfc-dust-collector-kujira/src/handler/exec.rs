use std::{collections::HashMap, iter::FromIterator};

//use crate::contract::{REPLY_RETURN, REPLY_SWAP};
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Response, Uint128,
    WasmMsg,
};
use kujira::Denom;
use pfc_dust_collector_kujira::{
    dust_collector::{MantaSellStrategy, SellStrategy},
    mantaswap,
};

//use crate::contract::{REPLY_RETURN, REPLY_SWAP};
use crate::error::ContractError;
use crate::{
    error::ContractError::MinMax,
    state::{ASSET_HOLDINGS, ASSET_HOLDINGS_MAX, ASSET_STAGES, CONFIG},
};

pub fn execute_set_asset_strategy(
    deps: DepsMut,
    sender: &Addr,
    denom: &Denom,
    strategy: &SellStrategy,
) -> Result<Response, ContractError> {
    if pfc_whitelist::is_listed(deps.storage, sender)?.is_some()
        || cw_ownable::is_owner(deps.storage, sender)?
    {
        ASSET_STAGES.save(deps.storage, denom.to_string(), strategy)?;

        let res = Response::new()
            .add_attribute("action", "set_asset_stages")
            .add_attribute("from", sender)
            .add_attribute("denom", denom.to_string());

        Ok(res)
    } else {
        Err(ContractError::Unauthorized {
            action: "sender is not on whitelist".to_string(),
            expected: "whitelist entry".to_string(),
            actual: "not on whitelist".to_string(),
        })
    }
}
pub fn execute_set_max_swaps(deps: DepsMut, max_swaps: u64) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    config.max_swaps = max_swaps;
    CONFIG.save(deps.storage, &config)?;
    let res = Response::new()
        .add_attribute("action", "set_max_swaps")
        .add_attribute("max_swaps", format!("{}", max_swaps));

    Ok(res)
}

pub fn execute_flushdust(
    deps: DepsMut,
    contract_address: &Addr,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    if pfc_whitelist::is_listed(deps.storage, &info.sender)?.is_some()
        || cw_ownable::is_owner(deps.storage, &info.sender)?
    {
        let funds_in: HashMap<String, Uint128> =
            HashMap::from_iter(info.funds.iter().map(|c| (c.denom.clone(), c.amount)));
        let swap_msgs = do_deposit(deps, contract_address, funds_in, true)?;

        Ok(Response::new()
            .add_attribute("action", "flush_dust")
            .add_attribute("from", info.sender)
            .add_attribute("action", "swap_and_send")
            .add_messages(swap_msgs))
    } else {
        Err(ContractError::Unauthorized {
            action: "sender is not on whitelist".to_string(),
            expected: "flush:false".to_string(),
            actual: "flush:true".to_string(),
        })
    }
}
pub fn execute_deposit(
    deps: DepsMut,
    contract_address: &Addr,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let funds_in: HashMap<String, Uint128> =
        HashMap::from_iter(info.funds.iter().map(|c| (c.denom.clone(), c.amount)));
    let swap_msgs = do_deposit(deps, contract_address, funds_in, false)?;
    Ok(Response::new()
        .add_attribute("action", "dust")
        .add_attribute("from", info.sender)
        .add_attribute("action", "swap_and_send")
        .add_messages(swap_msgs))
}

pub fn execute_set_asset_minimum(
    deps: DepsMut,
    sender: &Addr,
    denom: Denom,
    minimum: Uint128,
) -> Result<Response, ContractError> {
    if let Some(max) = ASSET_HOLDINGS_MAX.may_load(deps.storage, denom.to_string())? {
        if minimum > max {
            return Err(MinMax {
                min: minimum,
                max,
            });
        }
    }
    ASSET_HOLDINGS.save(deps.storage, denom.to_string(), &minimum)?;
    let res = Response::new()
        .add_attribute("action", "new_denom")
        .add_attribute("from", sender)
        .add_attribute("denom", denom.to_string())
        .add_attribute("minimum", format!("{}", minimum));

    Ok(res)
}
pub fn execute_set_asset_maximum(
    deps: DepsMut,
    sender: &Addr,
    denom: Denom,
    maximum: Uint128,
) -> Result<Response, ContractError> {
    if let Some(min) = ASSET_HOLDINGS.may_load(deps.storage, denom.to_string())? {
        if min > maximum {
            return Err(MinMax {
                min,
                max: maximum,
            });
        }
    }
    ASSET_HOLDINGS_MAX.save(deps.storage, denom.to_string(), &maximum)?;
    let res = Response::new()
        .add_attribute("action", "new_denom")
        .add_attribute("from", sender)
        .add_attribute("denom", denom.to_string())
        .add_attribute("maximum", format!("{}", maximum));

    Ok(res)
}

pub fn execute_set_base_denom(
    deps: DepsMut,
    sender: &Addr,
    denom: Denom,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    config.base_denom = denom.clone();

    CONFIG.save(deps.storage, &config)?;
    let res = Response::new()
        .add_attribute("action", "new_base_denom")
        .add_attribute("from", sender)
        .add_attribute("denom", denom.to_string());

    Ok(res)
}

pub fn execute_set_manta_token_router(
    deps: DepsMut,
    sender: &Addr,
    router: &str,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let router_addr = deps.api.addr_validate(router)?;
    config.manta_token_router = router_addr;

    CONFIG.save(deps.storage, &config)?;
    let res = Response::new()
        .add_attribute("action", "set_token_router")
        .add_attribute("from", sender)
        .add_attribute("token_router", router);

    Ok(res)
}
pub fn execute_set_calc_token_router(
    deps: DepsMut,
    sender: &Addr,
    router: &str,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let router_addr = deps.api.addr_validate(router)?;
    config.calc_token_router = router_addr;

    CONFIG.save(deps.storage, &config)?;
    let res = Response::new()
        .add_attribute("action", "set_token_router")
        .add_attribute("from", sender)
        .add_attribute("token_router", router);

    Ok(res)
}
pub fn execute_set_return_contract(
    deps: DepsMut,
    sender: &Addr,
    contract: &str,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let contract_addr = deps.api.addr_validate(contract)?;
    config.return_contract = contract_addr;

    CONFIG.save(deps.storage, &config)?;
    let res = Response::new()
        .add_attribute("action", "set_return_contract")
        .add_attribute("from", sender)
        .add_attribute("return", contract);

    Ok(res)
}

pub fn execute_clear_asset(
    deps: DepsMut,

    sender: &Addr,
    denom: Denom,
) -> Result<Response, ContractError> {
    if !ASSET_HOLDINGS.has(deps.storage, denom.to_string())
        && !ASSET_STAGES.has(deps.storage, denom.to_string())
    {
        return Err(ContractError::DenomNotFound {
            denom,
        });
    }
    ASSET_HOLDINGS.remove(deps.storage, denom.to_string());
    ASSET_STAGES.remove(deps.storage, denom.to_string());

    let res = Response::new()
        .add_attribute("action", "clear_asset")
        .add_attribute("from", sender)
        .add_attribute("denom", denom.to_string());

    Ok(res)
}

pub(crate) fn do_deposit(
    deps: DepsMut,
    contract_address: &Addr,
    _funds_in: HashMap<String, Uint128>,
    flush: bool,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    //   let to_denom = config.base_denom;
    let router = config.manta_token_router;
    let mut swap_msg_count: u64 = 0;
    let mut swaps: Vec<CosmosMsg> = Vec::new();

    let mut funds_to_swap: HashMap<String, Uint128> = HashMap::new(); //funds_in;

    let balances = deps.querier.query_all_balances(contract_address)?;
    // merge amounts
    for coin in balances {
        funds_to_swap
            .entry(coin.denom.clone())
            .and_modify(|e| *e += coin.amount)
            .or_insert(coin.amount);
    }
    for coin_balance in funds_to_swap.into_iter() {
        if let Some(minimum) = ASSET_HOLDINGS.may_load(deps.storage, coin_balance.0.clone())? {
            if coin_balance.1.ge(&minimum) || flush {
                // we don't have a maximum base_denom to send
                if coin_balance.0 == config.base_denom.to_string() {
                    let coin: Coin = Coin::new(coin_balance.1.u128(), coin_balance.0);
                    let contract_info =
                        deps.querier.query_wasm_contract_info(config.return_contract.to_string());

                    let return_msg = match contract_info {
                        Ok(_) => CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: config.return_contract.to_string(),
                            funds: vec![coin],
                            msg: to_json_binary(&pfc_steak::hub_tf::ExecuteMsg::ReturnDenom {})?,
                        }),
                        Err(_) => CosmosMsg::Bank(BankMsg::Send {
                            to_address: config.return_contract.to_string(),
                            amount: vec![coin],
                        }),
                    };

                    swaps.push(return_msg); //SubMsg::new(return_msg));
                } else if swap_msg_count <= config.max_swaps {
                    if let Some(strategy) =
                        ASSET_STAGES.may_load(deps.storage, coin_balance.0.clone())?
                    {
                        let maximum = ASSET_HOLDINGS_MAX
                            .may_load(deps.storage, coin_balance.0.clone())?
                            .unwrap_or(Uint128::MAX);
                        // cap amount to max if it is above it
                        let max_amt = coin_balance.1.min(maximum);
                        match strategy {
                            SellStrategy::Hold => {},
                            SellStrategy::Manta(mantaswap) => {
                                let swap = create_manta_swap_message(
                                    &router,
                                    &coin_balance.0,
                                    &mantaswap,
                                    max_amt,
                                )?;
                                swaps.push(swap); //SubMsg::reply_on_error(swap, REPLY_SWAP));
                                swap_msg_count += 1;
                            },
                            SellStrategy::Calc(_) => {},
                            SellStrategy::Airdrop(_) => {},
                            SellStrategy::Custom(_) => {},
                        }
                    }
                }
            }
        }
    }
    Ok(swaps)
}

/// this is a VERY basic swap.
fn create_manta_swap_message(
    router: &Addr,
    from_denom: &str,
    manta: &MantaSellStrategy,
    amount: Uint128,
) -> Result<CosmosMsg, ContractError> {
    let coin: Coin = Coin::new(amount.u128(), from_denom);

    let mut stages_to: Vec<Vec<(Addr, Denom)>> = vec![];
    for stage in &manta.stages {
        let mut stage_to: Vec<(Addr, Denom)> = vec![];
        for swap in stage {
            stage_to.push((swap.address.clone(), swap.denom.clone()))
        }
        stages_to.push(stage_to)
    }

    let swapmsg = mantaswap::ExecuteMsg::Swap {
        stages: stages_to,
        recipient: None,
        min_return: None,
    };
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: router.to_string(),
        funds: vec![coin],
        msg: to_json_binary(&swapmsg)?,
    }))
}
/*
pub(crate) fn execute_contract_reply(
    _deps: DepsMut,
    _env: Env,
    result: SubMsgResult,
) -> Result<Response, ContractError> {
    //let base_denom = CONFIG.load(deps.storage)?.base_denom;
    match result {
        /* shouldn't get here */
        SubMsgResult::Ok(_response) => {
             Ok(Response::default())
            /*
            for event in response.events {
                let offer_asset = event
                    .attributes
                    .into_iter()
                    .find(|p| p.key == "offer_asset")
                    .map(|f| f.value);
                /*    let return_amt = event
                .attributes
                .into_iter()
                .find(|p| p.key == "return_amount")
                .map(|f| f.value);*/
            }

             */
        }
        SubMsgResult::Err(error) => Err(ContractError::SubMessageFail { error }),
    }
}
*/
#[cfg(test)]
mod exec_test {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info},
        SubMsg,
    };
    use pfc_dust_collector_kujira::dust_collector::{
        AssetHolding, AssetMinimum, ConfigResponse, ExecuteMsg, QueryMsg, Stage,
    };

    use super::*;
    use crate::{
        contract::execute,
        querier::qry::query_helper,
        test_helpers::{
            do_instantiate, CREATOR, DENOM_1, DENOM_2, DENOM_3, DENOM_MAIN, LP_1, LP_2, LP_3,
            MANTA_ROUTER, USER_1, USER_2, USER_3, WL_USER_1,
        },
    };

    #[test]
    fn basic_init() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();

        do_instantiate(
            deps.as_mut(),
            CREATOR,
            vec![{
                AssetMinimum {
                    denom: DENOM_1.into(),
                    minimum: Uint128::from(10u128),
                }
            }],
            USER_1,
        )?;
        let res: ConfigResponse = query_helper(deps.as_ref(), QueryMsg::Config {});
        let expected = ConfigResponse {
            token_router: MANTA_ROUTER.to_string(),
            base_denom: Denom::from(DENOM_MAIN),
            return_contract: USER_1.to_string(),
            max_swaps: 2,
        };
        assert_eq!(expected, res, "Config is wrong");
        //eprintln!("{:?}", res);

        Ok(())
    }

    #[test]
    fn basic_swap() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();

        do_instantiate(
            deps.as_mut(),
            CREATOR,
            vec![{
                AssetMinimum {
                    denom: DENOM_1.into(),
                    minimum: Uint128::from(10_0000u128),
                }
            }],
            USER_1,
        )?;
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(USER_2, &[]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_1),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_1.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
        )
        .unwrap_err();
        match err {
            ContractError::Unauthorized {
                ..
            } => {},
            _ => unreachable!("wrong error {:?}", err),
        }
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(WL_USER_1, &[Coin::new(1_000, DENOM_1)]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_1),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_1.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
        )
        .unwrap_err();
        match err {
            ContractError::NoFundsRequired {
                ..
            } => {},
            _ => unreachable!("wrong error {:?}", err),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetMinimum {
                denom: Denom::from(DENOM_1),
                minimum: Uint128::from(1_000u128),
            },
        )?;
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(WL_USER_1, &[]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_1),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_1.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
        )?;

        let stages = query_helper::<AssetHolding>(
            deps.as_ref(),
            QueryMsg::Asset {
                denom: Denom::from(DENOM_1),
            },
        );
        assert_eq!(
            AssetHolding {
                denom: Denom::from(DENOM_1),
                minimum: Uint128::from(1_000u128),
                maximum: Uint128::MAX,
                balance: Uint128::zero(),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_1.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
            stages,
            "holding mismatch"
        );

        // ensure admin can change it
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_2),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_2.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
        )?;

        let stages = query_helper::<AssetHolding>(
            deps.as_ref(),
            QueryMsg::Asset {
                denom: Denom::from(DENOM_2),
            },
        );
        assert_eq!(
            AssetHolding {
                denom: Denom::from(DENOM_2),
                minimum: Uint128::zero(),
                maximum: Uint128::MAX,
                balance: Uint128::zero(),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_2.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
            stages,
            "holding mismatch"
        );

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(USER_2, &[Coin::new(500, DENOM_1)]),
            ExecuteMsg::DustReceived {},
        )?;

        assert!(res.messages.is_empty(), "no messages for this one");
        Ok(())
    }
    #[test]
    fn basic_swap2() -> Result<(), ContractError> {
        let mut deps = mock_dependencies_with_balance(&[Coin::new(1_000, DENOM_1)]);

        do_instantiate(
            deps.as_mut(),
            CREATOR,
            vec![{
                AssetMinimum {
                    denom: DENOM_1.into(),
                    minimum: Uint128::from(1_000u128),
                }
            }],
            USER_1,
        )?;
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(WL_USER_1, &[]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_1),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_1.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
        )?;

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(USER_2, &[Coin::new(1_000, DENOM_1)]),
            ExecuteMsg::DustReceived {},
        )?;
        //  eprintln!("{:?}", res);

        assert_eq!(res.messages.len(), 1, "1 swap pls");

        let submsg = res.messages.first().unwrap();
        let SubMsg {
            id: _,
            msg,
            gas_limit: _,
            reply_on: _,
        } = submsg;
        match msg {
            CosmosMsg::Wasm(wasmmsg) => {
                //eprintln!("WASM-MSG {:?}", wasmmsg);
                assert_eq!(
                    format!("{:?}", wasmmsg),
                    "Execute { contract_addr: \"manta_swap_contract\", msg: \
                     {\"swap\":{\"stages\":[[[\"LP_xyz_main\",\"umain\"]]],\"recipient\":null,\"\
                     min_return\":null}}, funds: [Coin { 1000 \"uxyz\" }] }",
                    "wrong message generated"
                )
            },
            _ => {
                assert_eq!(format!("{:?}", msg), "wrong type", "wrong type of message generated")
            },
        };

        Ok(())
    }
    #[test]
    pub fn swap_test_3() -> Result<(), ContractError> {
        let mut deps = mock_dependencies_with_balance(&[
            Coin::new(10_000, DENOM_1),
            Coin::new(10_000, DENOM_MAIN),
            Coin::new(100_000, DENOM_2),
        ]);

        do_instantiate(
            deps.as_mut(),
            CREATOR,
            vec![
                AssetMinimum {
                    denom: DENOM_1.into(),
                    minimum: Uint128::from(1_000u128),
                },
                AssetMinimum {
                    denom: DENOM_MAIN.into(),
                    minimum: Uint128::from(5_000u128),
                },
            ],
            USER_1,
        )?;
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(WL_USER_1, &[]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_1),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_1.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
        )?;
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_2),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_2.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
        )?;

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetMinimum {
                denom: Denom::from(DENOM_2),
                minimum: Uint128::from(200_000u128),
            },
        )?;
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetMinimum {
                denom: Denom::from(DENOM_MAIN),
                minimum: Uint128::from(5_000u128),
            },
        )?;
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(
                USER_3,
                &[
                    Coin::new(10_000, DENOM_1),
                    Coin::new(10_000, DENOM_MAIN),
                    Coin::new(100_000, DENOM_2),
                ],
            ),
            ExecuteMsg::DustReceived {},
        )?;
        assert_eq!(res.messages.len(), 2);

        let mut seen_bank_cnt: usize = 0;
        let mut seen_exec_cnt: usize = 0;
        for msg in &res.messages {
            match &msg.msg {
                CosmosMsg::Wasm(wasm) => {
                    seen_exec_cnt += 1;
                    match wasm {
                        WasmMsg::Execute {
                            contract_addr,
                            msg: _msg,
                            funds,
                        } => {
                            assert_eq!(MANTA_ROUTER, contract_addr);
                            assert_eq!(
                                "[Coin { 10000 \"uxyz\" }]",
                                format!("{:?}", funds),
                                "wrong amount"
                            );

                            assert_eq!(
                                "Execute { contract_addr: \"manta_swap_contract\", msg: \
                                 {\"swap\":{\"stages\":[[[\"LP_xyz_main\",\"umain\"]]],\"\
                                 recipient\":null,\"min_return\":null}}, funds: [Coin { 10000 \
                                 \"uxyz\" }] }",
                                format!("{:?}", wasm),
                                "wrong message?"
                            )
                        },
                        _ => {
                            eprintln!("{:?}", wasm);
                            panic!("invalid WASM message")
                        },
                    }
                },
                CosmosMsg::Bank(bank) => {
                    seen_bank_cnt += 1;
                    match bank {
                        BankMsg::Send {
                            to_address,
                            amount,
                        } => {
                            assert_eq!(to_address, USER_1, "wrong to address");
                            assert_eq!(
                                format!("{:?}", amount),
                                "[Coin { 10000 \"umain\" }]",
                                "wrong amount"
                            );
                        },
                        _ => {
                            eprintln!("{:?}", bank);
                            panic!("invalid bank message")
                        },
                    }
                },
                _ => {
                    unreachable!("unknown message type")
                },
            }
        }
        if seen_exec_cnt != 1 {
            eprintln!("{:?}", res.messages);
            assert_eq!(seen_exec_cnt, 1, "wrong number of Exec messages")
        }
        if seen_bank_cnt != 1 {
            eprintln!("{:?}", res.messages);
            assert_eq!(seen_bank_cnt, 1, "wrong number of Bank messages")
        }
        Ok(())
    }
    #[test]
    pub fn swap_test_4() -> Result<(), ContractError> {
        let mut deps = mock_dependencies_with_balance(&[
            Coin::new(10_000, DENOM_1),
            Coin::new(10_000, DENOM_MAIN),
            Coin::new(200_000, DENOM_2),
        ]);

        do_instantiate(
            deps.as_mut(),
            CREATOR,
            vec![
                AssetMinimum {
                    denom: DENOM_1.into(),
                    minimum: Uint128::from(1_000u128),
                },
                AssetMinimum {
                    denom: DENOM_MAIN.into(),
                    minimum: Uint128::from(5_000u128),
                },
            ],
            USER_1,
        )?;
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(WL_USER_1, &[]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_1),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_1.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
        )?;
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_2),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_2.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
        )?;

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetMinimum {
                denom: Denom::from(DENOM_2),
                minimum: Uint128::from(200_000u128),
            },
        )?;
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetMinimum {
                denom: Denom::from(DENOM_MAIN),
                minimum: Uint128::from(5_000u128),
            },
        )?;
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(
                USER_3,
                &[
                    Coin::new(10_000, DENOM_1),
                    Coin::new(10_000, DENOM_MAIN),
                    Coin::new(200_000, DENOM_2),
                ],
            ),
            ExecuteMsg::DustReceived {},
        )?;
        /*
        for msg in &res.messages {
            eprintln!("swap_test_4:{:?}", msg);
        }

         */

        assert_eq!(res.messages.len(), 3);

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetMinimum {
                denom: Denom::from(DENOM_3),
                minimum: Uint128::from(10_000u128),
            },
        )?;
        // multi-stage.
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_3),

                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![
                        Stage {
                            address: Addr::unchecked(LP_3.to_string()),
                            denom: Denom::from(DENOM_1),
                        },
                        Stage {
                            address: Addr::unchecked(LP_1.to_string()),
                            denom: Denom::from(DENOM_MAIN),
                        },
                    ]],
                }),
            },
        )?;
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[Coin::new(10_000, DENOM_3)]),
            ExecuteMsg::FlushDust {},
        )?;

        assert_eq!(res.messages.len(), 3);
        let denom_1_msg = res
            .messages
            .iter()
            .find(|sm| match sm.msg.clone() {
                CosmosMsg::Wasm(WasmMsg::Execute {
                    funds,
                    ..
                }) => funds[0].denom == DENOM_1,
                _ => false,
            })
            .unwrap();
        let denom_2_msg = res
            .messages
            .iter()
            .find(|sm| match sm.msg.clone() {
                CosmosMsg::Wasm(WasmMsg::Execute {
                    funds,
                    ..
                }) => funds[0].denom == DENOM_2,

                _ => false,
            })
            .unwrap();
        match denom_1_msg.msg.clone() {
            CosmosMsg::Wasm(WasmMsg::Execute {
                funds,
                ..
            }) => {
                assert_eq!(funds.len(), 1);
                let fund = funds[0].clone();
                assert_eq!(fund.denom, DENOM_1);
                assert_eq!(fund.amount, Uint128::from(10_000u32));
            },

            _ => panic!("expected a wasmMsg::exec"),
        }

        match denom_2_msg.msg.clone() {
            CosmosMsg::Wasm(WasmMsg::Execute {
                funds,
                ..
            }) => {
                assert_eq!(funds.len(), 1);
                let fund = funds[0].clone();
                assert_eq!(fund.denom, DENOM_2);
                assert_eq!(fund.amount, Uint128::from(200_000u32));
            },

            _ => {
                eprintln!("{:?}", res.messages[1].msg);
                panic!("expected a wasmMsg::exec")
            },
        }

        let stage = query_helper::<Option<AssetHolding>>(
            deps.as_ref(),
            QueryMsg::Asset {
                denom: Denom::from(DENOM_3),
            },
        );
        assert!(stage.is_some(), "should have an entry");

        let mut seen_bank_cnt: usize = 0;
        let mut seen_exec_cnt: usize = 0;
        for msg in &res.messages {
            match &msg.msg {
                CosmosMsg::Wasm(wasm) => {
                    seen_exec_cnt += 1;
                    match wasm {
                        WasmMsg::Execute {
                            contract_addr,
                            msg: _,
                            funds: _,
                        } => {
                            assert_eq!(MANTA_ROUTER, contract_addr);
                        },
                        _ => {
                            eprintln!("{:?}", wasm);
                            unreachable!("invalid WASM message")
                        },
                    }
                },
                CosmosMsg::Bank(bank) => {
                    seen_bank_cnt += 1;
                    match bank {
                        BankMsg::Send {
                            to_address,
                            amount,
                        } => {
                            assert_eq!(to_address, USER_1, "wrong to address");
                            assert_eq!(
                                format!("{:?}", amount),
                                "[Coin { 10000 \"umain\" }]",
                                "wrong amount"
                            );
                        },
                        _ => {
                            eprintln!("{:?}", bank);
                            unreachable!("invalid bank message")
                        },
                    }
                },
                _ => {
                    unreachable!("unknown message type")
                },
            }
        }
        if seen_exec_cnt != 2 {
            eprintln!("{:?}", res.messages);
            assert_eq!(seen_exec_cnt, 2, "wrong number of Exec messages")
        }
        if seen_bank_cnt != 1 {
            eprintln!("{:?}", res.messages);
            assert_eq!(seen_bank_cnt, 1, "wrong number of Bank messages")
        }
        Ok(())
    }
    #[test]
    // ensure max amounts are displayed, and only MAX amount is swapped.
    pub fn swap_test_maxes() -> Result<(), ContractError> {
        let mut deps = mock_dependencies_with_balance(&[
            Coin::new(10_000, DENOM_1),
            Coin::new(10_000, DENOM_MAIN),
            Coin::new(200_000, DENOM_2),
        ]);

        do_instantiate(
            deps.as_mut(),
            CREATOR,
            vec![
                AssetMinimum {
                    denom: DENOM_1.into(),
                    minimum: Uint128::from(1_000u128),
                },
                AssetMinimum {
                    denom: DENOM_MAIN.into(),
                    minimum: Uint128::from(5_000u128),
                },
            ],
            USER_1,
        )?;
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(WL_USER_1, &[]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_1),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_1.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
        )?;
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetStrategy {
                denom: Denom::from(DENOM_2),
                strategy: SellStrategy::Manta(MantaSellStrategy {
                    stages: vec![vec![Stage {
                        address: Addr::unchecked(LP_2.to_string()),
                        denom: Denom::from(DENOM_MAIN),
                    }]],
                }),
            },
        )?;

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetMinimum {
                denom: Denom::from(DENOM_2),
                minimum: Uint128::from(100_000u128),
            },
        )?;

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetMinimum {
                denom: Denom::from(DENOM_MAIN),
                minimum: Uint128::from(5_000u128),
            },
        )?;
        // denom 2 - max & min set
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetMaximum {
                denom: Denom::from(DENOM_2),
                maximum: Uint128::from(150_000u128),
            },
        )?;
        // denom 1 - no minimum, but a maximum
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &[]),
            ExecuteMsg::SetAssetMaximum {
                denom: Denom::from(DENOM_1),
                maximum: Uint128::from(5_000u128),
            },
        )?;
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(
                USER_3,
                &[
                    Coin::new(40_000, DENOM_1),
                    Coin::new(10_000, DENOM_MAIN),
                    Coin::new(600_000, DENOM_2),
                ],
            ),
            ExecuteMsg::DustReceived {},
        )?;
        /*
                for msg in &res.messages {
                    eprintln!("swap_test_max:{:?}", msg);
                }
        */
        assert_eq!(res.messages.len(), 3);
        let denom_1_msg = res
            .messages
            .iter()
            .find(|sm| match sm.msg.clone() {
                CosmosMsg::Wasm(WasmMsg::Execute {
                    funds,
                    ..
                }) => funds[0].denom == DENOM_1,
                _ => false,
            })
            .unwrap();
        let denom_2_msg = res
            .messages
            .iter()
            .find(|sm| match sm.msg.clone() {
                CosmosMsg::Wasm(WasmMsg::Execute {
                    funds,
                    ..
                }) => funds[0].denom == DENOM_2,
                _ => false,
            })
            .unwrap();
        match denom_1_msg.msg.clone() {
            CosmosMsg::Wasm(WasmMsg::Execute {
                funds,
                ..
            }) => {
                assert_eq!(funds.len(), 1);
                let fund = funds[0].clone();
                assert_eq!(fund.denom, DENOM_1);
                assert_eq!(fund.amount, Uint128::from(5_000u32));
            },

            _ => panic!("expected a wasmMsg::exec"),
        }
        match denom_2_msg.msg.clone() {
            CosmosMsg::Wasm(wasm) => match wasm {
                WasmMsg::Execute {
                    funds,
                    ..
                } => {
                    assert_eq!(funds.len(), 1);
                    let fund = funds[0].clone();
                    assert_eq!(fund.denom, DENOM_2);
                    assert_eq!(fund.amount, Uint128::from(150_000u32));
                },

                _ => panic!("expected a wasmMsg::exec"),
            },
            _ => {
                eprintln!("{:?}", res.messages[1].msg);
                panic!("expected a wasmMsg::exec")
            },
        }

        Ok(())
    }
}
