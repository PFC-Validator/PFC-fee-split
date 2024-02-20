use cosmwasm_std::{OverflowError, StdError, Uint128};
use cw_ownable::OwnershipError;
use kujira::Denom;
//use protobuf::ProtobufError;
use pfc_whitelist::WhitelistError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("PFC-Dust-Kujira: StdError:{0}")]
    Std(#[from] StdError),

    #[error("PFC-Dust-Kujira: Overflow:{0}")]
    Overflow(#[from] OverflowError),
    #[error("PFC-Dust-Kujira: Ownership:{0}")]
    Ownership(#[from] OwnershipError),
    #[error("PFC-Dust-Kujira: White list:{0}")]
    Whitelist(#[from] WhitelistError),
    #[error(
        "PFC-Dust-Kujira: Unauthorized (action: {action:?}, expected: {expected:?}, actual: \
         {actual:?})"
    )]
    Unauthorized {
        action: String,
        expected: String,
        actual: String,
    },
    #[error("PFC-Dust-Kujira: Recursion Send_type {send_type:?} - {contract:?}")]
    Recursion {
        send_type: String,
        contract: String,
    },

    #[error("PFC-Dust-Kujira: ExecuteError Failed - {action:?}")]
    ExecuteError {
        action: String,
    },
    #[error("PFC-Dust-Kujira: denom not found - {denom:?}")]
    DenomNotFound {
        denom: Denom,
    },
    #[error("PFC-Dust-Kujira: Denom already exists - {name:?}")]
    DenomAlreadyThere {
        name: Denom,
    },
    #[error("PFC-Dust-Kujira: Invalid Denom - {denom:?}")]
    InvalidDenom {
        denom: Denom,
    },
    #[error("PFC-Dust-Kujira: No fees are defined. Add one before sending deposits")]
    NoFeesError {},
    #[error("PFC-Dust-Kujira: Denom name must be unique")]
    DenomNotUnique {},
    #[error("PFC-Dust-Kujira: Don't send funds here")]
    NoFundsRequired {},
    #[error("PFC-Dust-Kujira: Min {min:?} > Max {max:?} ?")]
    MinMax {
        min: Uint128,
        max: Uint128,
    },

    #[error("PFC-Dust-Kujira: Contract can't be migrated! {current_name:?} {current_version:?}")]
    MigrationError {
        current_name: String,
        current_version: String,
    },
    #[error("PFC-Dust-Kujira: invalid reply {id} - {result}")]
    InvalidReply {
        id: u64,
        result: String,
    },
    #[error("PFC-Dust-Kujira: submessage fail - {error}")]
    SubMessageFail {
        error: String,
    },
}
