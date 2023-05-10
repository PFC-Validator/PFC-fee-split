use std::collections::HashSet;

use cosmwasm_std::{entry_point, Reply};
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};

use pfc_dust_collector_migaloo::dust_collector::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

use crate::error::ContractError;
use crate::handler::exec as ExecHandler;
use crate::handler::exec::{
    execute_clear_asset, execute_set_asset_minimum, execute_set_base_denom,
    execute_set_return_contract, execute_set_token_router,
};
use crate::handler::query as QueryHandler;
use crate::state;
use crate::state::{ASSET_HOLDINGS, CONFIG};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "pfc-dust-collector-migaloo";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const REPLY_SWAP: u64 = 42;
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;
    pfc_whitelist::initialize_whitelist(deps.storage, deps.api, msg.flush_whitelist)?;
    CONFIG.save(
        deps.storage,
        &state::Config {
            //this: deps.api.addr_validate(env.contract.address.as_str())?,
            token_router: deps.api.addr_validate(&msg.token_router)?,
            base_denom: msg.base_denom.clone(),
            return_contract: deps.api.addr_validate(&msg.return_contract)?,
        },
    )?;

    let dupe_check: HashSet<String> = msg.assets.iter().map(|v| v.denom.clone()).collect();
    if dupe_check.len() != msg.assets.len() {
        return Err(ContractError::DenomNotUnique {});
    }
    for row in msg.assets {
        if row.denom.trim().is_empty() || row.denom == msg.base_denom {
            return Err(ContractError::InvalidDenom { denom: row.denom });
        }
        ASSET_HOLDINGS.save(deps.storage, row.denom.clone(), &row.minimum)?
    }

    let mut res = Response::new();
    if let Some(hook) = msg.init_hook {
        res = res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: hook.contract_addr,
            msg: hook.msg,
            funds: vec![],
        }));
    }

    Ok(res)
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        REPLY_SWAP => ExecHandler::execute_contract_reply(deps, env, reply.result),
        id => Err(ContractError::InvalidReply {
            id,
            result: format!("{:?}", reply.result),
        }),
    }
}
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwnership(action) => {
            // this does admin checks internally
            cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::default())
        }
        ExecuteMsg::DustReceived {} => {
            ExecHandler::execute_deposit(deps, &env.contract.address, info)
        }
        ExecuteMsg::FlushDust {} => {
            ExecHandler::execute_flushdust(deps, &env.contract.address, info)
        }
        ExecuteMsg::SetReturnContract { contract } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            execute_set_return_contract(deps, &info.sender, &contract)
        }
        ExecuteMsg::SetTokenRouter { contract } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            execute_set_token_router(deps, &info.sender, &contract)
        }
        ExecuteMsg::SetBaseDenom { denom } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            execute_set_base_denom(deps, &info.sender, denom)
        }
        ExecuteMsg::SetAssetMinimum { denom, minimum } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            execute_set_asset_minimum(deps, &info.sender, denom, minimum)
        }
        ExecuteMsg::ClearAsset { denom } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            execute_clear_asset(deps, &info.sender, denom)
        }

        ExecuteMsg::AddToWhiteList { address, reason } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            pfc_whitelist::add_entry(deps.storage, deps.api, address, reason)?;
            Ok(Response::default())
        }
        ExecuteMsg::RemoveFromWhitelist { address } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            pfc_whitelist::remove_entry(deps.storage, deps.api, address)?;
            Ok(Response::default())
        }
    }
}
#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&QueryHandler::query_config(deps)?),
        QueryMsg::Ownership {} => to_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::Assets { .. } => {
            to_binary(&QueryHandler::query_assets(deps, &env.contract.address)?)
        }

        QueryMsg::Asset { denom } => to_binary(&QueryHandler::query_asset(
            deps,
            &env.contract.address,
            denom,
        )?),
        QueryMsg::Whitelist { start_after, limit } => to_binary(&pfc_whitelist::query_entries(
            deps.storage,
            start_after,
            limit,
        )?),
        QueryMsg::WhitelistEntry { address } => {
            to_binary(&pfc_whitelist::query_entry(deps.storage, address)?)
        }
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract_version = get_contract_version(deps.storage)?;

    match contract_version.contract.as_ref() {
        #[allow(clippy::single_match)]
        #[allow(clippy::match_single_binding)]
        "pfc-dust-collector-migaloo" => match &contract_version.version {
            _ => {}
        },
        _ => {
            return Err(ContractError::MigrationError {
                current_name: contract_version.contract,
                current_version: contract_version.version,
            })
        }
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("previous_contract_name", &contract_version.contract)
        .add_attribute("previous_contract_version", &contract_version.version)
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{CosmosMsg, SubMsg, WasmMsg};

    mod instantiate {
        use cosmwasm_std::{coin, Api, Binary};

        use pfc_fee_split::fee_split_msg::{
            AllocationDetail, AllocationHolding, InitHook, InstantiateMsg, SendType,
        };

        use crate::contract::instantiate;
        use crate::error::ContractError;
        use crate::handler::query::query_allocation;
        use crate::test_helpers::{
            one_allocation, ALLOCATION_1, ALLOCATION_2, CREATOR, DENOM_1, GOV_CONTRACT,
        };

        use super::*;

        #[test]
        fn basic() {
            let mut deps = mock_dependencies();
            let hook_msg = Binary::from(r#"{"some": 123}"#.as_bytes());
            let instantiate_msg = InstantiateMsg {
                name: "Hook Test".to_string(),

                init_hook: Some(InitHook {
                    contract_addr: String::from("hook_dest"),
                    msg: hook_msg.clone(),
                }),
                gov_contract: String::from(GOV_CONTRACT),
                allocation: one_allocation(&deps.api),
            };

            let info = mock_info(CREATOR, &[]);
            let env = mock_env();
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(
                res.messages,
                vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: String::from("hook_dest"),
                    msg: hook_msg,
                    funds: vec![],
                }))]
            );
            assert_eq!(
                query_allocation(deps.as_ref(), ALLOCATION_1.into())
                    .unwrap()
                    .unwrap(),
                AllocationHolding {
                    name: ALLOCATION_1.to_string(),
                    allocation: 1,
                    send_after: coin(1_000u128, DENOM_1),
                    send_type: SendType::Wallet {
                        receiver: deps.api.addr_validate("allocation_1_addr").unwrap()
                    },
                    balance: vec![]
                }
            );
            let instantiate_no_allocation_msg = InstantiateMsg {
                name: "FAIL".to_string(),
                init_hook: None,
                gov_contract: String::from(GOV_CONTRACT),
                allocation: vec![],
            };
            let info = mock_info(CREATOR, &[]);
            let env = mock_env();
            if let Some(err) =
                instantiate(deps.as_mut(), env, info, instantiate_no_allocation_msg).err()
            {
                match err {
                    ContractError::NoFeesError { .. } => {}
                    _ => assert!(false, "Invalid Error type {}", err),
                }
            } else {
                assert!(false, "should have failed")
            }
        }

        #[test]
        fn dupe_holdings() {
            let mut deps = mock_dependencies();
            let instantiate_msg = InstantiateMsg {
                name: "Dupe Allocation Test".to_string(),

                init_hook: None,
                gov_contract: String::from(GOV_CONTRACT),
                allocation: vec![
                    AllocationDetail {
                        name: ALLOCATION_1.to_string(),
                        allocation: 1,
                        send_after: coin(1_000u128, DENOM_1),
                        send_type: SendType::Wallet {
                            receiver: deps.api.addr_validate("allocation_1_addr").unwrap(),
                        },
                    },
                    AllocationDetail {
                        name: ALLOCATION_2.to_string(),
                        allocation: 1,
                        send_after: coin(1_0000_000u128, DENOM_1),
                        send_type: SendType::Wallet {
                            receiver: deps.api.addr_validate("allocation_2_addr").unwrap(),
                        },
                    },
                    AllocationDetail {
                        name: ALLOCATION_1.to_string(),
                        allocation: 3,
                        send_after: coin(1_0000_000u128, DENOM_1),
                        send_type: SendType::Wallet {
                            receiver: deps.api.addr_validate("allocation_3_addr").unwrap(),
                        },
                    },
                ],
            };

            let info = mock_info(CREATOR, &[]);
            let env = mock_env();
            let err = instantiate(deps.as_mut(), env, info, instantiate_msg)
                .err()
                .unwrap();
            match err {
                ContractError::FundAllocationNotUnique {} => {}
                _ => {
                    assert!(
                        false,
                        "this should have returned an FundAllocationNotUnique error"
                    )
                }
            }
        }
    }
}
