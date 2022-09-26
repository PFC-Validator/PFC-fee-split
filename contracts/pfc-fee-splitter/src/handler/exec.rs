use crate::error::ContractError;
use crate::state::{ADMIN, ALLOCATION_HOLDINGS, CONFIG};
use cosmwasm_std::{
    Addr, AllBalanceResponse, BankMsg, BankQuery, Coin, CosmosMsg, Decimal, DepsMut, Env,
    MessageInfo, Order, QuerierWrapper, QueryRequest, Response, StdError, StdResult, Uint128,
};
use pfc_fee_split::fee_split_msg::{AllocationHolding, SendType};
use std::collections::HashMap;
use std::iter::FromIterator;
use std::ops::Mul;
use std::str::FromStr;

pub fn execute_deposit(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    flush: bool,
) -> Result<Response, ContractError> {
    let funds_in: HashMap<String, Uint128> =
        HashMap::from_iter(info.funds.iter().map(|c| (c.denom.clone(), c.amount)));

    let total_allocation = get_total_weight(&deps)?;

    if info.funds.is_empty() && !flush {
        let res = Response::new()
            .add_attribute("action", "deposit")
            .add_attribute("from", info.sender)
            .add_attribute("no-action", "no funds sent, and flush false");

        return Ok(res);
    }

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
                allocation_holding.send_type,
                &allocation_holding.contract,
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
                        allocation_holding.send_type,
                        &allocation_holding.contract,
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

        // fee_holding.balance = vec![];
        ALLOCATION_HOLDINGS.save(deps.storage, key_name, &allocation_holding)?;
    }

    let res = Response::new()
        .add_attribute("action", "deposit")
        .add_attribute("from", info.sender)
        .add_messages(msgs);

    Ok(res)
}

pub fn execute_add_allocation_detail(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
    contract_unverified: String,
    allocation: u8,
    send_after: Coin,
    send_type_unverified: String,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let contract = deps.api.addr_validate(contract_unverified.as_str())?;
    let send_type =
        SendType::from_str(&send_type_unverified).map_err(|_| ContractError::SendTypeInvalid {
            send_type: send_type_unverified.clone(),
        })?;

    if allocation == 0 {
        return Err(ContractError::AllocationZero {});
    }
    if ALLOCATION_HOLDINGS.has(deps.storage, name.clone()) {
        return Err(ContractError::FeeAlreadyThere { name });
    }
    ALLOCATION_HOLDINGS.save(
        deps.storage,
        name.clone(),
        &AllocationHolding {
            name: name.clone(),
            contract: contract.clone(),
            send_type,
            send_after: send_after.clone(),
            allocation,

            balance: vec![],
        },
    )?;
    let res = Response::new()
        .add_attribute("action", "add_fee_detail")
        .add_attribute("from", info.sender)
        .add_attribute("name", name)
        .add_attribute("contract", contract.to_string())
        .add_attribute("allocation", format!("{}", allocation))
        .add_attribute("send_after", send_after.to_string())
        .add_attribute("send_type", send_type_unverified);
    Ok(res)
}
pub fn execute_modify_allocation_detail(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    name: String,
    contract_unverified: String,
    allocation: u8,
    send_after: Coin,
    send_type_unverified: String,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let contract = deps.api.addr_validate(contract_unverified.as_str())?;
    let send_type =
        SendType::from_str(&send_type_unverified).map_err(|_| ContractError::SendTypeInvalid {
            send_type: send_type_unverified.clone(),
        })?;
    if allocation == 0 {
        return Err(ContractError::AllocationZero {});
    }

    ALLOCATION_HOLDINGS.update(deps.storage, name.clone(), |rec| -> StdResult<_> {
        if let Some(mut fee_holding) = rec {
            fee_holding.contract = contract.clone();
            fee_holding.send_type = send_type;
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
        .add_attribute("contract", contract.to_string())
        .add_attribute("allocation", format!("{}", allocation))
        .add_attribute("send_after", send_after.to_string())
        .add_attribute("send_type", send_type_unverified);

    Ok(res)

    //return Err(ContractError::FeeNotFound { name: name.clone() });
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
        let msgs: Vec<CosmosMsg> = vec![generate_cosmos_msg(
            fee_holding.send_type,
            &fee_holding.contract,
            fee_holding.balance,
        )?];
        ALLOCATION_HOLDINGS.remove(deps.storage, name.clone());
        let res = Response::new()
            .add_attribute("action", "remove_fee_detail")
            .add_attribute("from", info.sender)
            .add_attribute("fee", name)
            .add_messages(msgs);

        Ok(res)
    } else {
        Err(ContractError::AllocationNotFound { name })
    }
}

pub fn execute_update_gov_contract(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    gov_contract: String,
) -> Result<Response, ContractError> {
    let admin = deps.api.addr_validate(&gov_contract)?;
    let mut config = CONFIG.load(deps.storage)?;
    config.gov_contract = admin.clone();
    CONFIG.save(deps.storage, &config)?;
    Ok(ADMIN.execute_update_admin(deps, info, Some(admin))?)
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
    //    eprintln!("fraction = {}", fraction);
    //    eprintln!("funds_sent = {:?}", funds_sent);
    let funds_sent_alloc: HashMap<String, Uint128> = funds_sent
        .iter()
        .map(|(denom, amount)| {
            let dec_amt: Decimal = Decimal::from_atomics(amount.u128(), 0).unwrap();
            let portion = dec_amt.mul(fraction);
            /*
                       eprintln!(
                           "dec_amt = {}, portion = {}",
                           dec_amt.to_string(),
                           portion.to_string()
                       );

            */
            if portion.is_zero() && portion > Decimal::from_ratio(1u32, 10_000u32) {
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
    //eprintln!("funds_sent_alloc = {:?}", funds_sent_alloc);
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

    Ok(send_coins)
}

fn generate_cosmos_msg(
    send_type: SendType,
    recipient: &Addr,
    coins: Vec<Coin>,
) -> Result<CosmosMsg, ContractError> {
    match send_type {
        SendType::WALLET => {
            let msg = BankMsg::Send {
                to_address: recipient.to_string(),
                amount: coins,
            };
            Ok(CosmosMsg::Bank(msg))
        }
    }
}

#[allow(dead_code)]
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
    use super::*;
    use cosmwasm_std::coin;

    use crate::contract::execute;
    use crate::handler::query::query_allocation;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use pfc_fee_split::fee_split_msg::ExecuteMsg;

    use crate::test_helpers::{
        do_instantiate, one_allocation, ALLOCATION_1, CREATOR, DENOM_1, DENOM_2, DENOM_3, USER_1,
    };
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
        let _res = do_instantiate(deps.as_mut(), CREATOR, one_allocation())?;
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
                amount: vec![coin(1_000_000, DENOM_1)]
            })
        );

        assert_eq!(res.attributes.len(), 2);
        let allocation = query_allocation(deps.as_ref(), ALLOCATION_1.into())?.unwrap();
        assert!(allocation.balance.is_empty(), "no coins should be present");

        Ok(())
    }
}
#[cfg(test)]
mod crud {
    use crate::contract::execute;
    use crate::error::ContractError;
    use crate::handler::query::{query_allocation, query_allocations};
    use crate::test_helpers::{
        do_instantiate, two_allocation, ALLOCATION_1, ALLOCATION_2, CREATOR, DENOM_1, GOV_CONTRACT,
        USER_1,
    };
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, Api, BankMsg, CosmosMsg, StdError};
    use pfc_fee_split::fee_split_msg::{AllocationHolding, ExecuteMsg, SendType};

    #[test]
    fn add_line() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();
        let _res = do_instantiate(deps.as_mut(), CREATOR, two_allocation())?;
        let allocations = query_allocations(deps.as_ref(), None, None)?;
        assert_eq!(allocations.allocations.len(), 2);
        assert_eq!(allocations.allocations[0].name, ALLOCATION_1);
        assert_eq!(allocations.allocations[1].name, ALLOCATION_2);
        let msg = ExecuteMsg::AddAllocationDetail {
            name: "line3".to_string(),
            contract: "line3-address".to_string(),
            allocation: 1,
            send_after: coin(0u128, DENOM_1),
            send_type: "Wallet".to_string(),
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
        let msg_duplicate = ExecuteMsg::AddAllocationDetail {
            name: ALLOCATION_2.to_string(),
            contract: "line3-address".to_string(),
            allocation: 1,
            send_after: coin(0u128, DENOM_1),
            send_type: "Wallet".to_string(),
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
                amount: vec![coin(333333, DENOM_1)]
            })
        );
        assert_eq!(
            res.messages[1].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "line3-address".to_string(),
                amount: vec![coin(333333, DENOM_1)]
            })
        );
        let allocations = query_allocation(deps.as_ref(), String::from(ALLOCATION_2))?.unwrap();

        assert_eq!(allocations.balance.len(), 1);
        assert_eq!(allocations.balance[0], coin(333_333, DENOM_1));

        Ok(())
    }
    #[test]
    fn rm_line() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();
        let _res = do_instantiate(deps.as_mut(), CREATOR, two_allocation())?;
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
                amount: vec![coin(1_000_000, DENOM_1)]
            })
        );

        let allocations = query_allocation(deps.as_ref(), String::from(ALLOCATION_1))?.unwrap();
        assert_eq!(allocations.balance.len(), 0);

        Ok(())
    }
    #[test]
    fn upd_line() -> Result<(), ContractError> {
        let mut deps = mock_dependencies();
        let _res = do_instantiate(deps.as_mut(), CREATOR, two_allocation())?;
        let allocations = query_allocations(deps.as_ref(), None, None)?;
        assert_eq!(allocations.allocations.len(), 2);
        assert_eq!(allocations.allocations[0].name, ALLOCATION_1);
        assert_eq!(allocations.allocations[1].name, ALLOCATION_2);
        let msg = ExecuteMsg::ModifyAllocationDetail {
            name: ALLOCATION_2.to_string(),
            contract: "new-contract".to_string(),
            allocation: 3,
            send_after: coin(1u128, DENOM_1),
            send_type: "Wallet".to_string(),
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
        let msg_does_not_exist = ExecuteMsg::ModifyAllocationDetail {
            name: "not-here".to_string(),
            contract: "new-contract".to_string(),
            allocation: 3,
            send_after: coin(1u128, DENOM_1),
            send_type: "Wallet".to_string(),
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
                amount: vec![coin(500_000, DENOM_1)]
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
                contract: deps.api.addr_validate("new-contract")?,
                allocation: 3,
                send_after: coin(1u128, DENOM_1),
                send_type: SendType::WALLET,
                balance: vec![coin(500_000, DENOM_1)]
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
                amount: vec![coin(250_000, DENOM_1)]
            })
        );
        assert_eq!(
            res.messages[1].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "new-contract".to_string(),
                amount: vec![coin(1_250_000, DENOM_1)]
            })
        );

        let allocations = query_allocation(deps.as_ref(), String::from(ALLOCATION_1))?.unwrap();
        assert_eq!(allocations.balance.len(), 0);

        Ok(())
    }
}
