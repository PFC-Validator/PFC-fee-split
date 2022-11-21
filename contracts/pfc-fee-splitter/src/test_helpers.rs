use cosmwasm_std::{coin, DepsMut, Response};

use cosmwasm_std::testing::{mock_env, mock_info};

use pfc_fee_split::fee_split_msg::{AllocationDetail, InstantiateMsg, SendType};

use crate::contract::instantiate;
use crate::error::ContractError;

pub const GOV_CONTRACT: &str = "gov_contract";
pub const CREATOR: &str = "creator";
pub const USER_1: &str = "user-0001";
pub const ALLOCATION_1: &str = "allocation_1";
pub const ALLOCATION_2: &str = "allocation_2";
pub const DENOM_1: &str = "uxyz";
pub const DENOM_2: &str = "uabc";
pub const DENOM_3: &str = "udef";

const NAME: &str = "pfc-fee-split";
pub(crate) fn do_instantiate(
    mut deps: DepsMut,
    addr: &str,
    allocation: Vec<AllocationDetail>,
) -> Result<Response, ContractError> {
    let instantiate_msg = InstantiateMsg {
        name: NAME.to_string(),
        gov_contract: GOV_CONTRACT.to_string(),
        allocation,
        init_hook: None,
    };
    let info = mock_info(addr, &[]);
    let env = mock_env();
    instantiate(deps.branch(), env, info, instantiate_msg)
}

pub(crate) fn one_allocation() -> Vec<AllocationDetail> {
    vec![AllocationDetail {
        name: ALLOCATION_1.to_string(),
        contract: "allocation_1_addr".to_string(),
        allocation: 1,
        send_after: coin(1_000u128, DENOM_1),
        send_type: SendType::WALLET,
    }]
}

pub(crate) fn two_allocation() -> Vec<AllocationDetail> {
    vec![
        AllocationDetail {
            name: ALLOCATION_1.to_string(),
            contract: "allocation_1_addr".to_string(),
            allocation: 1,
            send_after: coin(1_000u128, DENOM_1),
            send_type: SendType::WALLET,
        },
        AllocationDetail {
            name: ALLOCATION_2.to_string(),
            contract: "allocation_2_addr".to_string(),
            allocation: 1,
            send_after: coin(10_000_000u128, DENOM_1),
            send_type: SendType::SteakRewards {
                steak: String::from("steak_contract"),
                receiver: String::from("receiver"),
            },
        },
    ]
}
