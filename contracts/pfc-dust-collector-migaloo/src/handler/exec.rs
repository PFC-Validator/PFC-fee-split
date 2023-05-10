use std::collections::HashMap;
use std::iter::FromIterator;

use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, SubMsg, SubMsgResult,
    Uint128, WasmMsg,
};
use pool_network::asset::AssetInfo;
use pool_network::router::SwapOperation;

use crate::contract::REPLY_SWAP;
use crate::error::ContractError;
use crate::state::{ASSET_HOLDINGS, CONFIG};

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
        let swap_msg = do_deposit(deps, contract_address, funds_in, true)?;

        Ok(Response::new()
            .add_attribute("action", "flush_dust")
            .add_attribute("from", info.sender)
            .add_attribute("action", "swap_and_send")
            .add_submessages(swap_msg))
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
    if info.funds.is_empty() {
        // sometimes funds are empty.
        // if we errored out here, the calling transaction will fail.
        // so instead of forcing everyone calling us to make sure there is a fee, we just put a note

        let res = Response::new()
            .add_attribute("action", "dust_received")
            .add_attribute("from", info.sender)
            .add_attribute("no-action", "no funds sent, and flush false");

        return Ok(res);
    }

    let funds_in: HashMap<String, Uint128> =
        HashMap::from_iter(info.funds.iter().map(|c| (c.denom.clone(), c.amount)));
    let swap_msgs = do_deposit(deps, contract_address, funds_in, false)?;
    Ok(Response::new()
        .add_attribute("action", "dust")
        .add_attribute("from", info.sender)
        .add_attribute("action", "swap_and_send")
        .add_submessages(swap_msgs))
}

pub fn execute_set_asset_minimum(
    deps: DepsMut,
    sender: &Addr,
    denom: String,
    minimum: Uint128,
) -> Result<Response, ContractError> {
    let base_denom = CONFIG.load(deps.storage)?.base_denom;
    if denom == base_denom {
        Err(ContractError::InvalidDenom { denom })
    } else {
        ASSET_HOLDINGS.save(deps.storage, denom.clone(), &minimum)?;
        let res = Response::new()
            .add_attribute("action", "new_denom")
            .add_attribute("from", sender)
            .add_attribute("denom", denom)
            .add_attribute("minimum", format!("{}", minimum));

        Ok(res)
    }
}

pub fn execute_set_base_denom(
    deps: DepsMut,
    sender: &Addr,
    denom: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    config.base_denom = denom.clone();

    CONFIG.save(deps.storage, &config)?;
    let res = Response::new()
        .add_attribute("action", "new_base_denom")
        .add_attribute("from", sender)
        .add_attribute("denom", denom);

    Ok(res)
}
pub fn execute_set_token_router(
    deps: DepsMut,
    sender: &Addr,
    router: &str,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let router_addr = deps.api.addr_validate(router)?;
    config.token_router = router_addr;

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
    denom: String,
) -> Result<Response, ContractError> {
    if !ASSET_HOLDINGS.has(deps.storage, denom.clone()) {
        return Err(ContractError::DenomNotFound { denom });
    }
    ASSET_HOLDINGS.remove(deps.storage, denom.clone());

    let res = Response::new()
        .add_attribute("action", "clear_asset")
        .add_attribute("from", sender)
        .add_attribute("denom", denom);

    Ok(res)
}

pub(crate) fn do_deposit(
    deps: DepsMut,
    contract_address: &Addr,
    funds_in: HashMap<String, Uint128>,
    flush: bool,
) -> Result<Vec<SubMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let to_denom = config.base_denom;
    let router = config.token_router;
    let mut swaps: Vec<SubMsg> = Vec::new();

    let mut funds_to_swap: HashMap<String, Uint128> = funds_in;

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
                let swap =
                    create_swap_message(&router, &coin_balance.0, &to_denom, coin_balance.1)?;
                swaps.push(SubMsg::reply_on_success(swap, REPLY_SWAP));
            }
        }
    }
    Ok(swaps)
}

fn create_swap_message(
    router: &Addr,
    from_denom: &str,
    to_denom: &str,
    amount: Uint128,
) -> Result<CosmosMsg, ContractError> {
    let asset_in: AssetInfo = AssetInfo::NativeToken {
        denom: from_denom.to_string(),
    };
    let asset_out: AssetInfo = AssetInfo::NativeToken {
        denom: to_denom.to_string(),
    };
    let msg = pool_network::router::ExecuteMsg::ExecuteSwapOperation {
        operation: SwapOperation::TerraSwap {
            offer_asset_info: asset_in,
            ask_asset_info: asset_out,
        },
        to: None,
    };

    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: router.to_string(),
        msg: to_binary(&msg)?,
        funds: vec![Coin {
            denom: from_denom.into(),
            amount,
        }],
    }))
}

pub(crate) fn execute_contract_reply(
    deps: DepsMut,
    env: Env,
    result: SubMsgResult,
) -> Result<Response, ContractError> {
    let base_denom = CONFIG.load(deps.storage)?.base_denom;
    match result {
        SubMsgResult::Ok(response) => {
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
            todo!()
        }
        SubMsgResult::Err(error) => Err(ContractError::SubMessageFail { error }),
    }
}

#[cfg(test)]
mod exec {
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info,
    };

    use pfc_fee_split::fee_split_msg::ExecuteMsg;

    use crate::contract::execute;
    use crate::handler::query::{query_allocation, query_allocations};
    use crate::test_helpers::{
        do_instantiate, one_allocation, two_allocation, ALLOCATION_1, ALLOCATION_2, CREATOR,
        DENOM_1, DENOM_2, DENOM_3, GOV_CONTRACT, USER_1,
    };

    use super::*;

    #[test]
    fn allocations_1() -> Result<(), ContractError> {
        let zero = determine_allocation(1, 1, &HashMap::default(), &vec![])?;
        assert!(zero.is_empty(), "should have been empty");
        let funds: HashMap<String, Uint128> =
            HashMap::from([(DENOM_1.into(), Uint128::from(1_000_000u128))]);
        let full = determine_allocation(1, 1, &funds, &vec![])?;
        assert_eq!(full, vec![coin(1_000_000u128, String::from(DENOM_1))]);
        let tenth = determine_allocation(1, 10, &funds, &vec![])?;
        assert_eq!(tenth, vec![coin(100_000, String::from(DENOM_1))]);
        let third = determine_allocation(1, 3, &funds, &vec![])?;
        assert_eq!(third, vec![coin(333_333, String::from(DENOM_1))]);
        let three_quarters = determine_allocation(3, 4, &funds, &vec![])?;
        assert_eq!(three_quarters, vec![coin(750_000, String::from(DENOM_1))]);
        let funds2: HashMap<String, Uint128> = HashMap::from([
            (DENOM_1.into(), Uint128::from(1_000_000u128)),
            (DENOM_2.into(), Uint128::from(9_000u128)),
        ]);
        let two_parts = determine_allocation(3, 4, &funds2, &vec![])?;
        assert_eq!(
            two_parts.iter().find(|c| c.denom == DENOM_1).unwrap(),
            &coin(750_000, String::from(DENOM_1))
        );
        assert_eq!(
            two_parts.iter().find(|c| c.denom == DENOM_2).unwrap(),
            &coin(6_750, String::from(DENOM_2))
        );

        Ok(())
    }

    #[test]
    fn allocations_2() -> Result<(), ContractError> {
        let funds2: HashMap<String, Uint128> = HashMap::from([
            (DENOM_1.into(), Uint128::from(1_000_000u128)),
            (DENOM_2.into(), Uint128::from(9_000u128)),
        ]);
        let funds_held = vec![coin(100_000, String::from(DENOM_1))];
        let test_1 = determine_allocation(3, 4, &funds2, &funds_held)?;
        assert_eq!(
            test_1.iter().find(|c| c.denom == DENOM_1).unwrap(),
            &coin(850_000, String::from(DENOM_1))
        );
        assert_eq!(
            test_1.iter().find(|c| c.denom == DENOM_2).unwrap(),
            &coin(6_750, String::from(DENOM_2))
        );
        let funds_held = vec![
            coin(100_000, String::from(DENOM_1)),
            coin(20_000, String::from(DENOM_2)),
        ];

        let test_2 = determine_allocation(3, 4, &funds2, &funds_held)?;
        assert_eq!(
            test_2.iter().find(|c| c.denom == DENOM_1).unwrap(),
            &coin(850_000, String::from(DENOM_1))
        );
        assert_eq!(
            test_2.iter().find(|c| c.denom == DENOM_2).unwrap(),
            &coin(26_750, String::from(DENOM_2))
        );
        let funds_held = vec![
            coin(100_000, String::from(DENOM_1)),
            coin(20_000, String::from(DENOM_2)),
            coin(90_000, String::from(DENOM_3)),
        ];

        let test_3 = determine_allocation(3, 4, &funds2, &funds_held)?;
        assert_eq!(
            test_3.iter().find(|c| c.denom == DENOM_1).unwrap(),
            &coin(850_000, String::from(DENOM_1))
        );
        assert_eq!(
            test_3.iter().find(|c| c.denom == DENOM_2).unwrap(),
            &coin(26_750, String::from(DENOM_2))
        );
        assert_eq!(
            test_3.iter().find(|c| c.denom == DENOM_3).unwrap(),
            &coin(90_000, String::from(DENOM_3))
        );
        let funds_held_2 = vec![
            coin(100_000, String::from(DENOM_1)),
            coin(20_000, String::from(DENOM_2)),
        ];
        let funds3: HashMap<String, Uint128> = HashMap::from([
            (DENOM_1.into(), Uint128::from(1_000_000u128)),
            (DENOM_3.into(), Uint128::from(9_000u128)),
        ]);

        let test_4 = determine_allocation(3, 4, &funds3, &funds_held_2)?;
        assert_eq!(
            test_4.iter().find(|c| c.denom == DENOM_1).unwrap(),
            &coin(850_000, String::from(DENOM_1))
        );
        assert_eq!(
            test_4.iter().find(|c| c.denom == DENOM_2).unwrap(),
            &coin(20_000, String::from(DENOM_2))
        );
        assert_eq!(
            test_4.iter().find(|c| c.denom == DENOM_3).unwrap(),
            &coin(6_750, String::from(DENOM_3))
        );
        Ok(())
    }

    #[test]
    fn deposit_basic() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();
        let alloc = one_allocation(&deps.api);
        let _res = do_instantiate(deps.as_mut(), CREATOR, alloc)?;
        let msg = ExecuteMsg::Deposit { flush: false };
        let info = mock_info(USER_1, &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;

        assert_eq!(res.messages.len(), 0);
        assert_eq!(res.attributes.len(), 3);
        assert_eq!(res.attributes[2].key, "no-action");

        let info_with_funds = mock_info(USER_1, &[coin(1_000_000u128, String::from(DENOM_1))]);
        let res = execute(deps.as_mut(), env, info_with_funds, msg)?;
        //eprintln!("{:?}", res.messages[0]);
        assert_eq!(res.messages.len(), 1);
        assert_eq!(
            res.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "allocation_1_addr".to_string(),
                amount: vec![coin(1_000_000, DENOM_1)],
            })
        );

        assert_eq!(res.attributes.len(), 2);
        let allocation = query_allocation(deps.as_ref(), ALLOCATION_1.into())?.unwrap();
        assert!(allocation.balance.is_empty(), "no coins should be present");

        Ok(())
    }

    #[test]
    fn deposit_split() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();
        let allocs = two_allocation(&deps.api);

        let _res = do_instantiate(deps.as_mut(), CREATOR, allocs)?;
        let msg = ExecuteMsg::Deposit { flush: false };
        let info = mock_info(USER_1, &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;

        assert_eq!(res.messages.len(), 0);
        assert_eq!(res.attributes.len(), 3);
        assert_eq!(res.attributes[2].key, "no-action");

        let info_with_funds = mock_info(USER_1, &[coin(50_000_000u128, String::from(DENOM_1))]);
        let res = execute(deps.as_mut(), env, info_with_funds, msg)?;
        //eprintln!("{:?}", res.messages[0]);
        assert_eq!(res.messages.len(), 2);
        assert_eq!(
            res.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "allocation_1_addr".to_string(),
                amount: vec![coin(25_000_000, DENOM_1)],
            })
        );
        match &res.messages[1].msg {
            CosmosMsg::Wasm(wasmmsg) => match wasmmsg {
                WasmMsg::Execute {
                    contract_addr,
                    msg,
                    funds,
                } => {
                    assert_eq!(contract_addr, "steak_contract");
                    assert_eq!(funds.len(), 1);
                    assert_eq!(funds[0].amount, Uint128::new(25_000_000));
                    assert_eq!(funds[0].denom, DENOM_1);
                    let expected = to_binary(&pfc_steak::hub::ExecuteMsg::Bond {
                        receiver: Some(String::from("receiver")),
                    })?;
                    assert_eq!(msg, &expected)
                }
                _ => {
                    assert!(false, "Invalid MSG {:?}", res.messages[1].msg)
                }
            },
            _ => {
                assert!(false, "Invalid MSG {:?}", res.messages[1].msg)
            }
        }

        assert_eq!(res.attributes.len(), 2);
        let allocation = query_allocation(deps.as_ref(), ALLOCATION_1.into())?.unwrap();
        assert!(allocation.balance.is_empty(), "no coins should be present");

        Ok(())
    }

    #[test]
    fn reconcile_basic() -> Result<(), ContractError> {
        let mut deps = mock_dependencies_with_balance(&vec![
            Coin::new(1_000_000, DENOM_2),
            Coin::new(50_000, DENOM_1),
        ]);
        let alloc = two_allocation(&deps.api);
        let _res = do_instantiate(deps.as_mut(), CREATOR, alloc)?;
        let msg = ExecuteMsg::Reconcile {};
        let info = mock_info(USER_1, &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info = mock_info(GOV_CONTRACT, &[Coin::new(1_000, DENOM_1)]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::ReconcileWithFunds { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info = mock_info(GOV_CONTRACT, &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
        assert_eq!(res.attributes.len(), 1);
        assert_eq!(res.attributes[0].value, "reconcile");
        assert_eq!(res.messages.len(), 1);
        //  eprintln!("{:?}", res.messages[0]);
        match &res.messages[0].msg {
            CosmosMsg::Bank(b) => match b {
                BankMsg::Send { to_address, amount } => {
                    assert_eq!(to_address, "allocation_1_addr");
                    assert_eq!(amount.len(), 2);
                    assert_eq!(
                        amount.iter().find(|c| c.denom == DENOM_1),
                        Some(&coin(25_000, DENOM_1))
                    );
                    assert_eq!(
                        amount.iter().find(|c| c.denom == DENOM_2),
                        Some(&coin(500_000, DENOM_2))
                    )
                }
                _ => {
                    assert!(false, "invalid bank message {:?} ", b)
                }
            },
            _ => {
                assert!(false, "invalid message {:?} ", res.messages[0])
            }
        }

        let allocations = query_allocations(deps.as_ref(), None, None)?;
        assert_eq!(allocations.allocations.len(), 2);
        if allocations.allocations[0].name == ALLOCATION_1 {
            assert_eq!(allocations.allocations[0].name, ALLOCATION_1);
            assert_eq!(allocations.allocations[0].balance.is_empty(), true);
            assert_eq!(allocations.allocations[1].name, ALLOCATION_2);
            assert_eq!(allocations.allocations[1].balance.len(), 2);
            assert_eq!(
                allocations.allocations[1]
                    .balance
                    .iter()
                    .find(|c| c.denom == DENOM_2),
                Some(&coin(500_000, DENOM_2))
            );
            assert_eq!(
                allocations.allocations[1]
                    .balance
                    .iter()
                    .find(|c| c.denom == DENOM_1),
                Some(&coin(25_000, DENOM_1))
            );
        } else {
            assert_eq!(allocations.allocations[1].name, ALLOCATION_1);
            assert_eq!(allocations.allocations[1].balance.is_empty(), true);
            assert_eq!(allocations.allocations[0].name, ALLOCATION_2);
            assert_eq!(
                allocations.allocations[0]
                    .balance
                    .iter()
                    .find(|c| c.denom == DENOM_2),
                Some(&coin(500_000, DENOM_2))
            );
            assert_eq!(
                allocations.allocations[0]
                    .balance
                    .iter()
                    .find(|c| c.denom == DENOM_1),
                Some(&coin(25_000, DENOM_1))
            );
        }
        /* this will have to be tested on-chain. I don't think bank-sends actually debit in test
        // so at this point we should have 500k DENOM2 & 25k DENOM1. this test is to ensure we 'ignore' the existing balances, and send stuff out.

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
        assert_eq!(res.attributes.len(), 1);
        assert_eq!(res.attributes[0].value, "reconcile");
        assert_eq!(res.messages.len(), 1);
        //  eprintln!("{:?}", res.messages[0]);
        match &res.messages[0].msg {
            CosmosMsg::Bank(b) => match b {
                BankMsg::Send { to_address, amount } => {
                    assert_eq!(to_address, "allocation_1_addr");
                    assert_eq!(amount.len(), 2);
                    assert_eq!(
                        amount.iter().find(|c| c.denom == DENOM_1),
                        Some(&coin(12_500, DENOM_1))
                    );
                    assert_eq!(
                        amount.iter().find(|c| c.denom == DENOM_2),
                        Some(&coin(250_000, DENOM_2))
                    )
                }
                _ => {
                    assert!(false, "invalid bank message {:?} ", b)
                }
            },
            _ => {
                assert!(false, "invalid message {:?} ", res.messages[0])
            }
        }

        let allocations = query_allocations(deps.as_ref(), None, None)?;
        assert_eq!(allocations.allocations.len(), 2);
        if allocations.allocations[0].name == ALLOCATION_1 {
            assert_eq!(allocations.allocations[0].name, ALLOCATION_1);
            assert_eq!(allocations.allocations[0].balance.is_empty(), true);
            assert_eq!(allocations.allocations[1].name, ALLOCATION_2);
            assert_eq!(allocations.allocations[1].balance.len(), 2);
            assert_eq!(
                allocations.allocations[1].balance[0],
                coin(500_000, DENOM_2)
            );
            assert_eq!(allocations.allocations[1].balance[1], coin(25_000, DENOM_1));
        } else {
            assert_eq!(allocations.allocations[1].name, ALLOCATION_1);
            assert_eq!(allocations.allocations[1].balance.is_empty(), true);
            assert_eq!(allocations.allocations[0].name, ALLOCATION_2);
            assert_eq!(allocations.allocations[0].balance.len(), 2);
            assert_eq!(
                allocations.allocations[0].balance[0],
                coin(500_000, DENOM_2)
            );
            assert_eq!(allocations.allocations[0].balance[1], coin(25_000, DENOM_1));
        }

         */
        Ok(())
    }
}

#[cfg(test)]
mod crud_allocations {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, Api, BankMsg, CosmosMsg, StdError};

    use pfc_fee_split::fee_split_msg::{AllocationHolding, ExecuteMsg, SendType};

    use crate::contract::execute;
    use crate::error::ContractError;
    use crate::handler::query::{query_allocation, query_allocations};
    use crate::test_helpers::{
        do_instantiate, two_allocation, ALLOCATION_1, ALLOCATION_2, CREATOR, DENOM_1, GOV_CONTRACT,
        USER_1,
    };

    #[test]
    fn add_line() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();
        let alloc = two_allocation(&deps.api);
        let _res = do_instantiate(deps.as_mut(), CREATOR, alloc)?;
        let allocations = query_allocations(deps.as_ref(), None, None)?;
        assert_eq!(allocations.allocations.len(), 2);
        assert_eq!(allocations.allocations[0].name, ALLOCATION_1);
        assert_eq!(allocations.allocations[1].name, ALLOCATION_2);
        let msg = ExecuteMsg::AddAllocationDetail {
            name: "line3".to_string(),

            allocation: 1,
            send_after: coin(0u128, DENOM_1),
            send_type: SendType::SteakRewards {
                steak: deps.api.addr_validate("steak-contract")?,
                receiver: deps.api.addr_validate("rewards")?,
            },
        };
        //eprintln!("{}", serde_json::to_string(&msg).unwrap());
        let info = mock_info(USER_1, &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info = mock_info(CREATOR, &[]);

        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info = mock_info(GOV_CONTRACT, &[]);
        let msg_duplicate = ExecuteMsg::AddAllocationDetail {
            name: ALLOCATION_2.to_string(),

            allocation: 1,
            send_after: coin(0u128, DENOM_1),
            send_type: SendType::SteakRewards {
                steak: deps.api.addr_validate("steak-contract")?,
                receiver: deps.api.addr_validate("rewards")?,
            },
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            msg_duplicate.clone(),
        )
        .err()
        .unwrap();
        match err {
            ContractError::FeeAlreadyThere { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
        assert!(res.attributes.len() > 0, "attributes should be present");

        let allocations = query_allocations(deps.as_ref(), None, None)?;
        assert_eq!(allocations.allocations.len(), 3);
        assert_eq!(allocations.allocations[0].name, ALLOCATION_1);
        assert_eq!(allocations.allocations[1].name, ALLOCATION_2);
        assert_eq!(allocations.allocations[2].name, "line3");

        let msg = ExecuteMsg::Deposit { flush: false };
        let info = mock_info(USER_1, &[coin(1_000_000, DENOM_1)]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
        assert_eq!(res.attributes.len(), 2);
        assert_eq!(res.messages.len(), 2);
        assert_eq!(
            res.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "allocation_1_addr".to_string(),
                amount: vec![coin(333333, DENOM_1)],
            })
        );
        assert_eq!(
            &format!("{:?}", res.messages[1].msg),
            "Wasm(Execute { contract_addr: \"steak-contract\", msg: {\"bond\":{\"receiver\":\"rewards\"}}, funds: [Coin { denom: \"uxyz\", amount: Uint128(333333) }] })"
        );
        let allocations = query_allocation(deps.as_ref(), String::from(ALLOCATION_2))?.unwrap();

        assert_eq!(allocations.balance.len(), 1);
        assert_eq!(allocations.balance[0], coin(333_333, DENOM_1));

        Ok(())
    }

    #[test]
    fn rm_line() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();
        let alloc = two_allocation(&deps.api);
        let _res = do_instantiate(deps.as_mut(), CREATOR, alloc)?;
        let allocations = query_allocations(deps.as_ref(), None, None)?;
        assert_eq!(allocations.allocations.len(), 2);
        assert_eq!(allocations.allocations[0].name, ALLOCATION_1);
        assert_eq!(allocations.allocations[1].name, ALLOCATION_2);
        let msg = ExecuteMsg::RemoveAllocationDetail {
            name: ALLOCATION_2.to_string(),
        };
        let info = mock_info(USER_1, &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info = mock_info(CREATOR, &[]);

        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info = mock_info(GOV_CONTRACT, &[]);
        let msg_does_not_exist = ExecuteMsg::RemoveAllocationDetail {
            name: "does-not-exist".to_string(),
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            msg_does_not_exist.clone(),
        )
        .err()
        .unwrap();
        match err {
            ContractError::AllocationNotFound { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
        assert!(res.attributes.len() > 0, "attributes should be present");

        let allocations = query_allocations(deps.as_ref(), None, None)?;
        assert_eq!(allocations.allocations.len(), 1);
        assert_eq!(allocations.allocations[0].name, ALLOCATION_1);

        let msg = ExecuteMsg::Deposit { flush: false };
        let info = mock_info(USER_1, &[coin(1_000_000, DENOM_1)]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
        assert_eq!(res.attributes.len(), 2);
        assert_eq!(res.messages.len(), 1);
        assert_eq!(
            res.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "allocation_1_addr".to_string(),
                amount: vec![coin(1_000_000, DENOM_1)],
            })
        );

        let allocations = query_allocation(deps.as_ref(), String::from(ALLOCATION_1))?.unwrap();
        assert_eq!(allocations.balance.len(), 0);

        Ok(())
    }

    #[test]
    fn upd_line() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();
        let alloc = two_allocation(&deps.api);
        let _res = do_instantiate(deps.as_mut(), CREATOR, alloc)?;
        let allocations = query_allocations(deps.as_ref(), None, None)?;
        assert_eq!(allocations.allocations.len(), 2);
        assert_eq!(allocations.allocations[0].name, ALLOCATION_1);
        assert_eq!(allocations.allocations[1].name, ALLOCATION_2);
        let msg = ExecuteMsg::ModifyAllocationDetail {
            name: ALLOCATION_2.to_string(),
            allocation: 3,
            send_after: coin(1u128, DENOM_1),
            send_type: SendType::Wallet {
                receiver: deps.api.addr_validate("new-contract").unwrap(),
            },
        };
        //eprintln!("{}", serde_json::to_string(&msg).unwrap());
        let info = mock_info(USER_1, &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info = mock_info(CREATOR, &[]);

        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info = mock_info(GOV_CONTRACT, &[]);
        let msg_does_not_exist = ExecuteMsg::ModifyAllocationDetail {
            name: "not-here".to_string(),
            allocation: 3,
            send_after: coin(1u128, DENOM_1),
            send_type: SendType::Wallet {
                receiver: deps.api.addr_validate("new-contract").unwrap(),
            },
        };
        let err = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            msg_does_not_exist.clone(),
        )
        .err()
        .unwrap();
        match err {
            ContractError::Std(x) => match x {
                StdError::NotFound { .. } => {}
                _ => assert!(false, "wrong std error {:?}", x),
            },
            _ => assert!(false, "wrong error {:?}", err),
        }

        let msg_deposit = ExecuteMsg::Deposit { flush: false };
        let info = mock_info(USER_1, &[coin(1_000_000, DENOM_1)]);
        let env = mock_env();
        let res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            msg_deposit.clone(),
        )?;
        assert_eq!(res.attributes.len(), 2);
        assert_eq!(res.messages.len(), 1);
        assert_eq!(
            res.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "allocation_1_addr".to_string(),
                amount: vec![coin(500_000, DENOM_1)],
            })
        );
        let allocation = query_allocation(deps.as_ref(), String::from(ALLOCATION_2))?.unwrap();
        assert_eq!(allocation.balance.len(), 1);
        assert_eq!(allocation.balance[0], coin(500_000, DENOM_1));

        // do the update
        let info = mock_info(GOV_CONTRACT, &[]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
        assert!(res.attributes.len() > 0, "attributes should be present");

        let allocations = query_allocations(deps.as_ref(), None, None)?;
        assert_eq!(allocations.allocations.len(), 2);
        assert_eq!(allocations.allocations[0].name, ALLOCATION_1);
        assert_eq!(allocations.allocations.len(), 2);
        assert_eq!(allocations.allocations[1].name, ALLOCATION_2);
        assert_eq!(
            allocations.allocations[1],
            AllocationHolding {
                name: ALLOCATION_2.to_string(),

                allocation: 3,
                send_after: coin(1u128, DENOM_1),
                send_type: SendType::Wallet {
                    receiver: deps.api.addr_validate("new-contract").unwrap()
                },
                balance: vec![coin(500_000, DENOM_1)],
            }
        );

        let msg_deposit = ExecuteMsg::Deposit { flush: false };
        let info = mock_info(USER_1, &[coin(1_000_000, DENOM_1)]);
        let env = mock_env();
        let res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            msg_deposit.clone(),
        )?;
        assert_eq!(res.attributes.len(), 2);
        assert_eq!(res.messages.len(), 2);
        assert_eq!(
            res.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "allocation_1_addr".to_string(),
                amount: vec![coin(250_000, DENOM_1)],
            })
        );
        assert_eq!(
            res.messages[1].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "new-contract".to_string(),
                amount: vec![coin(1_250_000, DENOM_1)],
            })
        );

        let allocations = query_allocation(deps.as_ref(), String::from(ALLOCATION_1))?.unwrap();
        assert_eq!(allocations.balance.len(), 0);

        Ok(())
    }
}

#[cfg(test)]
mod flush_whitelist {
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use pfc_fee_split::fee_split_msg::ExecuteMsg;

    use crate::contract::execute;
    use crate::error::ContractError;
    use crate::handler::query::query_flush_whitelist;
    use crate::test_helpers::{
        do_instantiate, one_allocation, two_allocation, CREATOR, DENOM_1, GOV_CONTRACT, USER_1,
    };

    #[test]
    fn add_remove_whitelists() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();
        let alloc = two_allocation(&deps.api);
        let _res = do_instantiate(deps.as_mut(), CREATOR, alloc)?;
        let msg = ExecuteMsg::AddToFlushWhitelist {
            address: "johnny".to_string(),
        };
        let info = mock_info(USER_1, &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        // the one creating it has no admin privs
        let info = mock_info(CREATOR, &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }

        let info = mock_info(GOV_CONTRACT, &[]);
        let env = mock_env();
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
        let msg2 = ExecuteMsg::AddToFlushWhitelist {
            address: "jimmy".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg2.clone())?;
        // yes.. johnny was added twice intentionally\
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
        let mut whitelist = query_flush_whitelist(deps.as_ref())?.allowed;
        whitelist.sort();
        assert_eq!(whitelist.len(), 2);
        assert_eq!(whitelist.get(0).unwrap(), "jimmy");
        assert_eq!(whitelist.get(1).unwrap(), "johnny");

        let msg = ExecuteMsg::RemoveFromFlushWhitelist {
            address: "johnny".to_string(),
        };
        let info = mock_info(USER_1, &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }

        // the one creating it has no admin privs
        let info = mock_info(CREATOR, &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())
            .err()
            .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }

        let info = mock_info(GOV_CONTRACT, &[]);
        let env = mock_env();
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
        let msg2 = ExecuteMsg::RemoveFromFlushWhitelist {
            address: "jason".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg2.clone())?;
        // yes.. johnny was added twice intentionally\
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone())?;
        let mut whitelist = query_flush_whitelist(deps.as_ref())?.allowed;
        whitelist.sort();
        assert_eq!(whitelist.len(), 1);
        assert_eq!(whitelist.get(0).unwrap(), "jimmy");

        Ok(())
    }

    #[test]
    fn flush_deposit() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();
        let alloc = one_allocation(&deps.api);
        let _res = do_instantiate(deps.as_mut(), CREATOR, alloc)?;
        let info = mock_info(GOV_CONTRACT, &[]);
        let env = mock_env();
        let msg_add = ExecuteMsg::AddToFlushWhitelist {
            address: "jimmy".to_string(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg_add.clone())?;
        let info_with_funds = mock_info(USER_1, &[coin(1_000_000u128, String::from(DENOM_1))]);
        let msg_no_flush = ExecuteMsg::Deposit { flush: false };
        let msg_flush = ExecuteMsg::Deposit { flush: true };
        let res = execute(
            deps.as_mut(),
            env.clone(),
            info_with_funds.clone(),
            msg_no_flush,
        )?;
        assert_eq!(res.messages.len(), 1);

        let err = execute(
            deps.as_mut(),
            env.clone(),
            info_with_funds.clone(),
            msg_flush.clone(),
        )
        .err()
        .unwrap();
        match err {
            ContractError::Unauthorized { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info_auth_with_funds =
            mock_info("jimmy", &[coin(1_000_000u128, String::from(DENOM_1))]);

        let res = execute(
            deps.as_mut(),
            env.clone(),
            info_auth_with_funds.clone(),
            msg_flush,
        )?;
        assert_eq!(res.messages.len(), 1);

        Ok(())
    }
}

#[cfg(test)]
mod ownership_changes {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use pfc_fee_split::fee_split_msg::ExecuteMsg;

    use crate::contract::execute;
    use crate::error::ContractError;
    use crate::test_helpers::{do_instantiate, two_allocation, CREATOR, GOV_CONTRACT, USER_1};

    #[test]
    fn change_owners() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();
        let alloc = two_allocation(&deps.api);
        let _res = do_instantiate(deps.as_mut(), CREATOR, alloc)?;
        let msg_gov_transfer = ExecuteMsg::TransferGovContract {
            gov_contract: "new_gov".to_string(),
            blocks: 1000,
        };
        let info = mock_info(USER_1, &[]);
        let env = mock_env();
        let err = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            msg_gov_transfer.clone(),
        )
        .err()
        .unwrap();
        match err {
            ContractError::AdminError { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info = mock_info(GOV_CONTRACT, &[]);
        let _res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            msg_gov_transfer.clone(),
        )?;
        let msg_flush = ExecuteMsg::Deposit { flush: true };

        //  not admin yet
        let info = mock_info("new_gov", &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg_flush.clone())
            .err()
            .unwrap();
        match err {
            ContractError::Unauthorized { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }

        // old gov still good
        let info = mock_info(GOV_CONTRACT, &[]);
        let env = mock_env();
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg_flush.clone())?;

        let msg_accept_gov_transfer = ExecuteMsg::AcceptGovContract {};
        let env = mock_env();
        let info = mock_info(USER_1, &[]);
        let err = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            msg_accept_gov_transfer.clone(),
        )
        .err()
        .unwrap();
        match err {
            ContractError::Unauthorized { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info = mock_info(GOV_CONTRACT, &[]);
        let err = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            msg_accept_gov_transfer.clone(),
        )
        .err()
        .unwrap();
        match err {
            ContractError::Unauthorized { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }
        let info = mock_info("new_gov", &[]);
        let _res = execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            msg_accept_gov_transfer.clone(),
        )?;

        // no longer admin
        let info = mock_info(GOV_CONTRACT, &[]);

        let env = mock_env();
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg_flush.clone())
            .err()
            .unwrap();
        match err {
            ContractError::Unauthorized { .. } => {}
            _ => assert!(false, "wrong error {:?}", err),
        }

        // new gov is good
        let info = mock_info("new_gov", &[]);
        let env = mock_env();
        let _res = execute(deps.as_mut(), env.clone(), info.clone(), msg_flush.clone())?;

        Ok(())
    }
}
