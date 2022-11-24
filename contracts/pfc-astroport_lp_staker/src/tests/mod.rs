use cosmwasm_std::{Attribute, CosmosMsg, SubMsg, WasmMsg};

pub mod bond;
pub mod instantiate;
pub mod unbond;
pub mod update_config;
pub mod validate;
pub mod withdraw;

pub fn find_attribute<'a>(attributes: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
    attributes.iter().find(|f| f.key == name)
}
pub fn find_exec(message: &SubMsg) -> Option<&WasmMsg> {
    match &message.msg {
        CosmosMsg::Wasm(wasm) => match wasm {
            WasmMsg::Execute { .. } => Some(wasm),
            _ => None,
        },
        _ => None,
    }
}
