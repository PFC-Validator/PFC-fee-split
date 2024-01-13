use cosmwasm_std::{DepsMut, Response};

use cosmwasm_std::testing::{mock_env, mock_info};
use kujira::Denom;
use pfc_dust_collector_kujira::dust_collector::{AssetMinimum, InstantiateMsg};
use pfc_whitelist::Whitelist;

use crate::contract::instantiate;
use crate::error::ContractError;

//pub const GOV_CONTRACT: &str = "gov_contract";
pub const CREATOR: &str = "creator";
pub const MANTA_ROUTER: &str = "manta_swap_contract";
pub const CALC_ROUTER: &str = "calc_swap_contract";
pub const USER_1: &str = "user-0001";
pub const USER_2: &str = "user-0002";
pub const USER_3: &str = "user-0003";
pub const WL_USER_1: &str = "alice";
pub const WL_USER_2: &str = "bob";
pub const DENOM_MAIN: &str = "umain";
pub const DENOM_1: &str = "uxyz";
pub const DENOM_2: &str = "uabc";
pub const DENOM_3: &str = "udef";
pub const LP_1: &str = "LP_xyz_main";
pub const LP_2: &str = "LP_abc_xyz";
pub const LP_3: &str = "LP_def_xyz";

pub(crate) fn do_instantiate(
    mut deps: DepsMut,
    addr: &str,
    assets: Vec<AssetMinimum>,
    return_to: &str,
) -> Result<Response, ContractError> {
    let instantiate_msg = InstantiateMsg {
        owner: CREATOR.to_string(),
        manta_token_router: MANTA_ROUTER.to_string(),
        calc_token_router: CALC_ROUTER.to_string(),
        return_contract: return_to.to_string(),
        base_denom: Denom::from(DENOM_MAIN),
        assets,
        max_swaps: 2,
        flush_whitelist: vec![
            Whitelist {
                address: WL_USER_1.to_string(),
                reason: Some("Alice Reason".to_string()),
            },
            Whitelist {
                address: WL_USER_2.to_string(),
                reason: None,
            },
        ],

        init_hook: None,
    };
    let info = mock_info(addr, &[]);
    let env = mock_env();
    instantiate(deps.branch(), env, info, instantiate_msg)
}
