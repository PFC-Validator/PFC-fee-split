use std::collections::HashSet;

use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use kujira::Denom;

use pfc_dust_collector_kujira::dust_collector::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SellStrategy,
};

use crate::error::ContractError;
use crate::handler::exec as ExecHandler;
use crate::handler::exec::{
    execute_clear_asset, execute_set_asset_maximum, execute_set_asset_minimum,
    execute_set_asset_strategy, execute_set_base_denom, execute_set_calc_token_router,
    execute_set_manta_token_router, execute_set_max_swaps, execute_set_return_contract,
};
use crate::handler::query as QueryHandler;
use crate::state;
use crate::state::{ASSET_HOLDINGS, ASSET_STAGES, CONFIG};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "pfc-dust-collector-kujira";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

//pub const REPLY_SWAP: u64 = 69;
//pub const REPLY_RETURN: u64 = 420;
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
            manta_token_router: deps.api.addr_validate(&msg.manta_token_router)?,
            calc_token_router: deps.api.addr_validate(&msg.calc_token_router)?,
            base_denom: msg.base_denom.clone(),
            return_contract: deps.api.addr_validate(&msg.return_contract)?,
            max_swaps: msg.max_swaps,
        },
    )?;

    let dupe_check: HashSet<Denom> = msg.assets.iter().map(|v| v.denom.clone()).collect();
    if dupe_check.len() != msg.assets.len() {
        return Err(ContractError::DenomNotUnique {});
    }
    for row in msg.assets {
        //  if row.denom.trim().is_empty() || row.denom == msg.base_denom {
        //      return Err(ContractError::InvalidDenom { denom: row.denom });
        //  }

        ASSET_HOLDINGS.save(deps.storage, row.denom.to_string(), &row.minimum)?;
        ASSET_STAGES.save(
            deps.storage,
            row.denom.to_string(),
            &SellStrategy::default(),
        )?;
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
/*
#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        REPLY_SWAP => ExecHandler::execute_contract_reply(deps, env, reply.result),
        REPLY_RETURN => ExecHandler::execute_contract_reply(deps, env, reply.result),
        id => Err(ContractError::InvalidReply {
            id,
            result: format!("{:?}", reply.result),
        }),
    }
}
*/
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    if !info.funds.is_empty() {
        match &msg {
            ExecuteMsg::DustReceived { .. } | ExecuteMsg::FlushDust { .. } => {}
            _ => return Err(ContractError::NoFundsRequired {}),
        }
    }
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
        ExecuteMsg::SetMantaTokenRouter { contract } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            execute_set_manta_token_router(deps, &info.sender, &contract)
        }
        ExecuteMsg::SetCalcTokenRouter { contract } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            execute_set_calc_token_router(deps, &info.sender, &contract)
        }
        ExecuteMsg::SetBaseDenom { denom } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            execute_set_base_denom(deps, &info.sender, denom)
        }
        ExecuteMsg::SetAssetMinimum { denom, minimum } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            execute_set_asset_minimum(deps, &info.sender, denom, minimum)
        }
        ExecuteMsg::SetAssetMaximum { denom, maximum } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            execute_set_asset_maximum(deps, &info.sender, denom, maximum)
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
        ExecuteMsg::SetAssetStrategy { denom, strategy } => {
            execute_set_asset_strategy(deps, &info.sender, &denom, &strategy)
        }
        ExecuteMsg::SetMaxSwaps { max_swaps } => {
            cw_ownable::assert_owner(deps.storage, &info.sender)?;
            execute_set_max_swaps(deps, max_swaps)
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
        "pfc-dust-collector-kujira" => match &contract_version.version {
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
        use cosmwasm_std::{Binary, Uint128};
        use kujira::Denom;

        use pfc_dust_collector_kujira::dust_collector::{
            AssetHolding, AssetMinimum, CollectorResponse, InitHook, InstantiateMsg, QueryMsg,
            SellStrategy,
        };
        use pfc_whitelist::Whitelist;

        use crate::contract::instantiate;
        use crate::error::ContractError;
        use crate::querier::qry::query_helper;
        use crate::test_helpers::{CREATOR, DENOM_1, DENOM_2, DENOM_3, DENOM_MAIN};

        use super::*;

        #[test]
        fn basic() {
            let mut deps = mock_dependencies();
            let hook_msg = Binary::from(r#"{"some": 123}"#.as_bytes());
            let instantiate_msg = InstantiateMsg {
                owner: "owner".to_string(),
                manta_token_router: "swap".to_string(),
                calc_token_router: "calc".to_string(),
                return_contract: "jim".to_string(),
                base_denom: Denom::from(DENOM_3),
                assets: vec![
                    AssetMinimum {
                        denom: Denom::from(DENOM_1),
                        minimum: Uint128::from(10u64),
                    },
                    AssetMinimum {
                        denom: Denom::from(DENOM_2),
                        minimum: Uint128::from(20u64),
                    },
                ],
                max_swaps: 1,
                flush_whitelist: vec![
                    Whitelist {
                        address: "alice".to_string(),
                        reason: Some("Alice Reason".to_string()),
                    },
                    Whitelist {
                        address: "bob".to_string(),
                        reason: Some("Bob Reason".to_string()),
                    },
                ],
                init_hook: Some(InitHook {
                    contract_addr: String::from("hook_dest"),
                    msg: hook_msg.clone(),
                }),
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
            let stages = query_helper::<CollectorResponse<AssetHolding>>(
                deps.as_ref(),
                QueryMsg::Assets {
                    start_after: None,
                    limit: None,
                },
            );

            let mut seen_denom_1 = 0;
            let mut seen_denom_2 = 0;
            for entry in &stages.entries {
                match entry.denom.to_string().as_str() {
                    DENOM_1 => {
                        seen_denom_1 += 1;
                        assert_eq!(entry.minimum, Uint128::from(10u128))
                    }
                    DENOM_2 => {
                        seen_denom_2 += 1;
                        assert_eq!(entry.minimum, Uint128::from(20u128))
                    }
                    _ => {
                        unreachable!("{} not expected", entry.denom)
                    }
                }
                assert_eq!(entry.strategy, SellStrategy::Hold);
                assert_eq!(entry.balance, Uint128::zero());
            }
            assert_eq!(seen_denom_1, 1, "wantred to see DENOM_1 once");
            assert_eq!(seen_denom_2, 1, "wantred to see DENOM_2 once");
            assert_eq!(2, stages.entries.len(), "should have 2 entries");

            let denom_2 = stages
                .entries
                .iter()
                .find(|p| p.denom.to_string() == DENOM_2)
                .unwrap();
            assert_eq!(denom_2.strategy, SellStrategy::Hold, "should be hold");
            assert_eq!(
                denom_2.minimum,
                Uint128::from(20u128),
                "denom2 minimum should be 20"
            );

            let instantiate_msg_fail = InstantiateMsg {
                owner: "owner".to_string(),
                manta_token_router: "swap".to_string(),
                calc_token_router: "calc".to_string(),
                return_contract: "jim".to_string(),
                base_denom: Denom::from(DENOM_3),
                assets: vec![
                    AssetMinimum {
                        denom: Denom::from(DENOM_1),
                        minimum: Uint128::from(10u64),
                    },
                    AssetMinimum {
                        denom: Denom::from(DENOM_2),
                        minimum: Uint128::from(20u64),
                    },
                    AssetMinimum {
                        denom: Denom::from(DENOM_1),
                        minimum: Uint128::from(30u64),
                    },
                ],
                max_swaps: 1,
                flush_whitelist: vec![
                    Whitelist {
                        address: "alice".to_string(),
                        reason: Some("Alice Reason".to_string()),
                    },
                    Whitelist {
                        address: "bob".to_string(),
                        reason: Some("Bob Reason".to_string()),
                    },
                ],
                init_hook: None,
            };
            let info = mock_info(CREATOR, &[]);
            let env = mock_env();
            let err = instantiate(deps.as_mut(), env, info, instantiate_msg_fail).unwrap_err();
            match err {
                ContractError::DenomNotUnique {} => {}
                _ => unreachable!("wrong error {:?}", err),
            }
            let instantiate_msg_2 = InstantiateMsg {
                owner: "owner".to_string(),
                manta_token_router: "swap".to_string(),
                calc_token_router: "calc".to_string(),
                return_contract: "jim".to_string(),
                base_denom: Denom::from(DENOM_MAIN),
                assets: vec![
                    AssetMinimum {
                        denom: Denom::from(DENOM_1),
                        minimum: Uint128::from(10u64),
                    },
                    AssetMinimum {
                        denom: Denom::from(DENOM_2),
                        minimum: Uint128::from(20u64),
                    },
                    AssetMinimum {
                        denom: Denom::from(DENOM_MAIN),
                        minimum: Uint128::from(20u64),
                    },
                ],
                max_swaps: 1,
                flush_whitelist: vec![
                    Whitelist {
                        address: "alice".to_string(),
                        reason: Some("Alice Reason".to_string()),
                    },
                    Whitelist {
                        address: "bob".to_string(),
                        reason: Some("Bob Reason".to_string()),
                    },
                ],
                init_hook: None,
            };
            let info = mock_info(CREATOR, &[]);
            let env = mock_env();
            instantiate(deps.as_mut(), env, info, instantiate_msg_2).unwrap();
            /*
            match err {
                ContractError::InvalidDenom { .. } => {}

                _ => assert!(false, "wrong error {:?}", err),
            }

             */
        }
    }
}
