use std::collections::HashMap;
use std::iter::FromIterator;
use std::ops::Mul;

use cosmwasm_std::{
    to_binary, Addr, AllBalanceResponse, BankMsg, BankQuery, Coin, CosmosMsg, Decimal, DepsMut,
    Env, MessageInfo, Order, QuerierWrapper, QueryRequest, Response, StdError, StdResult, Uint128,
    WasmMsg,
};

use pfc_fee_split::fee_split_msg::{AllocationHolding, SendType};

use crate::error::ContractError;
use crate::state::{ADMIN, ALLOCATION_HOLDINGS, CONFIG, FLUSH_WHITELIST};

pub fn execute_deposit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    flush: bool,
) -> Result<Response, ContractError> {
    if info.funds.is_empty() && !flush {
        // sometimes funds are empty.
        // if we errored out here, the calling transaction will fail.
        // so instead of forcing everyone calling us to make sure there is a fee, we just put a note

        let res = Response::new()
            .add_attribute("action", "deposit")
            .add_attribute("from", info.sender)
            .add_attribute("no-action", "no funds sent, and flush false");

        return Ok(res);
    }
    if flush
        && !FLUSH_WHITELIST.contains(deps.storage, info.sender.clone())
        && !ADMIN.is_admin(deps.as_ref(), &info.sender)?
    {
        return Err(ContractError::Unauthorized {
            action: "sender is not on whitelist".to_string(),
            expected: "flush:false".to_string(),
            actual: "flush:true".to_string(),
        });
    }

    let funds_in: HashMap<String, Uint128> =
        HashMap::from_iter(info.funds.iter().map(|c| (c.denom.clone(), c.amount)));
    let msgs = do_deposit(deps, funds_in, flush)?;

    let res = Response::new()
        .add_attribute("action", "deposit")
        .add_attribute("from", info.sender)
        .add_messages(msgs);

    Ok(res)
}

pub fn execute_add_allocation_detail(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    // contract_unverified: String,
    allocation: u8,
    send_after: Coin,
    send_type_unverified: SendType,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    //let contract = deps.api.addr_validate(contract_unverified.as_str())?;
    if !send_type_unverified.verify(&env.contract.address) {
        return Err(ContractError::Recursion {
            send_type: send_type_unverified.to_string(),
            contract: env.contract.address.to_string(),
        });
    }

    if allocation == 0 {
        return Err(ContractError::AllocationZero {});
    }

    if send_after.denom.trim().is_empty() {
        return Err(ContractError::InvalidCoin { coin: send_after });
    }

    if ALLOCATION_HOLDINGS.has(deps.storage, name.clone()) {
        return Err(ContractError::FeeAlreadyThere { name });
    }
    ALLOCATION_HOLDINGS.save(
        deps.storage,
        name.clone(),
        &AllocationHolding {
            name: name.clone(),
            send_type: send_type_unverified.clone(),
            send_after: send_after.clone(),
            allocation,

            balance: vec![],
        },
    )?;
    let res = Response::new()
        .add_attribute("action", "add_fee_detail")
        .add_attribute("from", info.sender)
        .add_attribute("name", name)
        .add_attribute("allocation", format!("{}", allocation))
        .add_attribute("send_after", send_after.to_string())
        .add_attribute("send_type", send_type_unverified.to_string());
    Ok(res)
}

pub fn execute_modify_allocation_detail(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    name: String,
    allocation: u8,
    send_after: Coin,
    send_type_unverified: SendType,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    //send_type_unverified.verify(deps.api)?;
    if !send_type_unverified.verify(&env.contract.address) {
        return Err(ContractError::Recursion {
            send_type: send_type_unverified.to_string(),
            contract: env.contract.address.to_string(),
        });
    }

    if allocation == 0 {
        return Err(ContractError::AllocationZero {});
    }

    ALLOCATION_HOLDINGS.update(deps.storage, name.clone(), |rec| -> StdResult<_> {
        if let Some(mut fee_holding) = rec {
            fee_holding.send_type = send_type_unverified.clone();
            fee_holding.send_after = send_after.clone();
            fee_holding.allocation = allocation;
            Ok(fee_holding)
        } else {
            Err(StdError::NotFound {
                kind: name.to_string(),
            })
        }
    })?;

    let res = Response::new()
        .add_attribute("action", "modify_fee_detail")
        .add_attribute("from", info.sender)
        .add_attribute("name", name)
        .add_attribute("allocation", format!("{}", allocation))
        .add_attribute("send_after", send_after.to_string())
        .add_attribute("send_type", send_type_unverified.to_string());

    Ok(res)
}

pub fn execute_remove_allocation_detail(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    if ALLOCATION_HOLDINGS
        .keys(deps.storage, None, None, Order::Ascending)
        .count()
        <= 1
    {
        return Err(ContractError::NoFeesError {});
    }
    if let Some(fee_holding) = ALLOCATION_HOLDINGS.may_load(deps.storage, name.clone())? {
        let balances = fee_holding
            .balance
            .into_iter()
            .filter(|f| f.amount > Uint128::zero())
            .collect();
        let msgs: Vec<CosmosMsg> = vec![generate_cosmos_msg(fee_holding.send_type, balances)?];
        ALLOCATION_HOLDINGS.remove(deps.storage, name.clone());

        let res = Response::new()
            .add_attribute("action", "remove_fee_detail")
            .add_attribute("from", info.sender)
            .add_attribute("fee", name);
        if msgs.is_empty() {
            Ok(res)
        } else {
            Ok(res.add_messages(msgs))
        }
    } else {
        Err(ContractError::AllocationNotFound { name })
    }
}

pub fn execute_add_flush_whitelist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let address_addr = deps.api.addr_validate(address.as_str())?;
    FLUSH_WHITELIST.insert(deps.storage, address_addr)?;
    let res = Response::new()
        .add_attribute("action", "add_flush_whitelist")
        .add_attribute("from", info.sender);

    Ok(res)
}

pub fn execute_remove_flush_whitelist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let address_addr = deps.api.addr_validate(address.as_str())?;
    if FLUSH_WHITELIST.contains(deps.storage, address_addr.clone()) {
        FLUSH_WHITELIST.remove(deps.storage, address_addr)?;

        let res = Response::new()
            .add_attribute("action", "remove_flush_whitelist")
            .add_attribute("from", info.sender);

        Ok(res)
    } else {
        let res = Response::new()
            .add_attribute("action", "remove_flush_whitelist")
            .add_attribute("from", info.sender)
            .add_attribute("not_there", address);

        Ok(res)
    }
}

pub fn execute_reconcile(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    if !info.funds.is_empty() {
        return Err(ContractError::ReconcileWithFunds {});
    }
    let keys = ALLOCATION_HOLDINGS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    if keys.is_empty() {
        return Err(ContractError::NoFeesError {});
    }
    for key in keys {
        ALLOCATION_HOLDINGS.update(deps.storage, key.clone(), |rec| {
            if let Some(mut record) = rec {
                record.balance = vec![];
                Ok(record)
            } else {
                Err(StdError::NotFound { kind: key })
            }
        })?;
    }
    let funds = get_native_balances(&deps.querier, env.contract.address)?;
    if funds.is_empty() {
        return Ok(Response::new()
            .add_attribute("action", "reconcile")
            .add_attribute("info", "no funds. clearing balances"));
    }
    let funds_in: HashMap<String, Uint128> =
        HashMap::from_iter(funds.iter().map(|c| (c.denom.clone(), c.amount)));

    let msgs = do_deposit(deps, funds_in, false)?;
    let res = Response::new()
        .add_attribute("action", "reconcile")
        .add_messages(msgs);
    Ok(res)
}

pub fn execute_update_gov_contract(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    gov_contract: String,
    blocks: u64,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let new_admin = deps.api.addr_validate(&gov_contract)?;
    let mut config = CONFIG.load(deps.storage)?;
    config.new_gov_contract = Some(new_admin);
    config.change_gov_contract_by_height = Some(env.block.height + blocks);

    CONFIG.save(deps.storage, &config)?;
    let res = Response::new().add_attribute("action", "update_gov_contract");
    Ok(res)
}

pub fn execute_accept_gov_contract(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if let Some(new_admin) = config.new_gov_contract {
        if new_admin != info.sender {
            Err(ContractError::Unauthorized {
                action: "accept_gov_contract".to_string(),
                expected: new_admin.to_string(),
                actual: info.sender.to_string(),
            })
        } else if let Some(block_height) = config.change_gov_contract_by_height {
            if block_height < env.block.height {
                Err(ContractError::Unauthorized {
                    action: "accept_gov_contract expired".to_string(),
                    expected: format!("{}", block_height),
                    actual: format!("{}", env.block.height),
                })
            } else {
                config.gov_contract = new_admin.clone();
                config.new_gov_contract = None;
                config.change_gov_contract_by_height = None;
                CONFIG.save(deps.storage, &config)?;
                ADMIN.set(deps, Some(new_admin))?;
                let res = Response::new().add_attribute("action", "accept_gov_contract");
                Ok(res)
            }
        } else {
            Err(ContractError::Unauthorized {
                action: "accept_gov_contract no height".to_string(),
                expected: "-missing-".to_string(),
                actual: format!("{}", env.block.height),
            })
        }
    } else {
        Err(ContractError::Unauthorized {
            action: "accept_gov_contract not set".to_string(),
            expected: "-missing-".to_string(),
            actual: "-missing-".to_string(),
        })
    }
}

pub(crate) fn get_total_weight(deps: &DepsMut) -> Result<u8, ContractError> {
    Ok(ALLOCATION_HOLDINGS
        .range(deps.storage, None, None, Order::Ascending)
        .fold(0, |acc, x| acc + x.unwrap().1.allocation))
}

///
/// this function takes the allocation ratio (allocation_amt & total_allocation)
/// and first splits funds_sent by that allocation
/// it then merges in the funds_held (as-is)
///
/// returns: Vec<Coin> - amount after deposit
pub(crate) fn determine_allocation(
    allocation_amt: u8,
    total_allocation: u8,
    funds_sent: &HashMap<String, Uint128>,
    funds_held: &[Coin],
) -> Result<Vec<Coin>, ContractError> {
    let fraction = Decimal::from_ratio(allocation_amt, total_allocation);
    let funds_sent_alloc: HashMap<String, Uint128> = funds_sent
        .iter()
        .map(|(denom, amount)| {
            let dec_amt: Decimal = Decimal::from_atomics(amount.u128(), 0).unwrap();
            let portion = dec_amt.mul(fraction);

            // ignore dust
            if portion.is_zero() || portion < Decimal::from_ratio(1u32, 10_000u32) {
                (denom.clone(), Uint128::zero())
            } else {
                let places = portion.decimal_places();
                let portion_u128 = portion
                    .atomics()
                    .checked_div(Uint128::from(10u32).pow(places))
                    .unwrap();
                (denom.clone(), portion_u128)
            }
        })
        .collect();

    let bal: HashMap<String, Uint128> = funds_held
        .iter()
        .map(|c| {
            if let Some(amt) = funds_sent_alloc.get(&c.denom) {
                (c.denom.clone(), c.amount + amt)
            } else {
                (c.denom.clone(), c.amount)
            }
        })
        .collect();
    //eprintln!("bal1 = {:?}", bal);
    let send_coins = bal
        .iter()
        .chain(
            funds_sent_alloc
                .iter()
                .filter(|(denom, _amount)| !bal.contains_key(&(*denom).clone())),
        )
        .map(|(denom, amount)| Coin::new(u128::from(*amount), denom))
        .collect::<Vec<Coin>>();
    let send_coins_non_dust = send_coins
        .into_iter()
        .filter(|p| p.amount > Uint128::from(100u32))
        .collect();
    Ok(send_coins_non_dust)
}

pub(crate) fn do_deposit(
    deps: DepsMut,
    funds_in: HashMap<String, Uint128>,
    flush: bool,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let total_allocation = get_total_weight(&deps)?;

    let mut msgs: Vec<CosmosMsg> = Vec::new();

    let keys = ALLOCATION_HOLDINGS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect::<Vec<_>>();
    if keys.is_empty() {
        return Err(ContractError::NoFeesError {});
    }
    for key in keys {
        let key_name = key?;
        let mut allocation_holding = ALLOCATION_HOLDINGS.load(deps.storage, key_name.clone())?;

        let merged_coins = determine_allocation(
            allocation_holding.allocation,
            total_allocation,
            &funds_in,
            &allocation_holding.balance,
        )?;
        if flush {
            msgs.push(generate_cosmos_msg(
                allocation_holding.send_type.clone(),
                merged_coins,
            )?);
            allocation_holding.balance = vec![];
        } else {
            let check_coin = merged_coins
                .iter()
                .find(|c| c.denom == allocation_holding.send_after.denom);
            if let Some(coin) = check_coin {
                if coin.amount > allocation_holding.send_after.amount {
                    msgs.push(generate_cosmos_msg(
                        allocation_holding.send_type.clone(),
                        merged_coins,
                    )?);
                    allocation_holding.balance = vec![];
                } else {
                    allocation_holding.balance = merged_coins;
                }
            } else {
                allocation_holding.balance = merged_coins;
            }
        }

        ALLOCATION_HOLDINGS.save(deps.storage, key_name, &allocation_holding)?;
    }
    Ok(msgs)
}

fn generate_cosmos_msg(send_type: SendType, coins: Vec<Coin>) -> Result<CosmosMsg, ContractError> {
    match send_type {
        SendType::Wallet { receiver } => {
            let msg = BankMsg::Send {
                to_address: receiver.to_string(),
                amount: coins,
            };
            Ok(CosmosMsg::Bank(msg))
        }
        SendType::SteakRewards { steak, receiver } => {
            let msg = pfc_steak::hub::ExecuteMsg::Bond {
                receiver: Some(receiver.to_string()),
            };
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: steak.to_string(),
                msg: to_binary(&msg)?,
                funds: coins,
            }))
        }
    }
}

pub(crate) fn get_native_balances(
    querier: &QuerierWrapper,
    account_addr: Addr,
) -> StdResult<Vec<Coin>> {
    let balances: AllBalanceResponse =
        querier.query(&QueryRequest::Bank(BankQuery::AllBalances {
            address: account_addr.to_string(),
        }))?;
    Ok(balances.amount)
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

    #[test]
    fn zero_funds() -> Result<(), ContractError> {
        // Small amount of funds sent
        let funds: HashMap<String, Uint128> =
            HashMap::from([(DENOM_1.into(), Uint128::from(99u128))]);

        // 1 allocation is very small
        let merged_coins = determine_allocation(1, 99, &funds, &vec![])?;

        let addr_a = Addr::unchecked("a");
        let addr_b = Addr::unchecked("b");

        let send_type = SendType::SteakRewards {
            steak: addr_a.clone(),
            receiver: addr_b.clone(),
        };

        // Given a 0 amount coin, generate_cosmos_msg should return an error but it constructs a valid CosmosMsg
        let msg = generate_cosmos_msg(send_type, merged_coins);

        // Message with 0 amount is generated, and will error
        assert_ne!(
            msg.unwrap(),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr_a.to_string(),
                msg: to_binary(&pfc_steak::hub::ExecuteMsg::Bond {
                    receiver: Some(addr_b.to_string()),
                })
                .unwrap(),
                funds: vec![coin(0, String::from(DENOM_1))],
            })
        );

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
