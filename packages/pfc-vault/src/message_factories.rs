use cosmwasm_std::{to_json_binary, Addr, Binary, Coin, CosmosMsg, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use serde::Serialize;

pub fn cw20_transfer(token: &Addr, recipient: &Addr, amount: Uint128) -> CosmosMsg {
    CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: token.to_string(),
        funds: vec![],
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount,
        })
        .unwrap(),
    })
}

pub fn wasm_execute<T>(contract: &Addr, msg: &T) -> CosmosMsg
where
    T: Serialize + ?Sized,
{
    wasm_execute_bin(contract, to_json_binary(&msg).unwrap())
}

pub fn wasm_execute_with_funds<T>(contract: &Addr, funds: Vec<Coin>, msg: &T) -> CosmosMsg
where
    T: Serialize + ?Sized,
{
    wasm_execute_bin_with_funds(contract, funds, to_json_binary(msg).unwrap())
}

pub fn wasm_execute_bin(contract: &Addr, msg: Binary) -> CosmosMsg {
    wasm_execute_bin_with_funds(contract, vec![], msg)
}

pub fn wasm_execute_bin_with_funds(contract: &Addr, funds: Vec<Coin>, msg: Binary) -> CosmosMsg {
    CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        funds,
        msg,
    })
}
