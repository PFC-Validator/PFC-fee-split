use cosmwasm_std::{OverflowError, StdError};
use cw_ownable::OwnershipError;
use thiserror::Error;

//use protobuf::ProtobufError;
use pfc_whitelist::WhitelistError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("PFC-Dust-Migaloo: StdError:{0}")]
    Std(#[from] StdError),

    #[error("PFC-Dust-Migaloo: Overflow:{0}")]
    Overflow(#[from] OverflowError),
    #[error("PFC-Dust-Migaloo: Ownership:{0}")]
    Ownership(#[from] OwnershipError),
    #[error("PFC-Dust-Migaloo: White list:{0}")]
    Whitelist(#[from] WhitelistError),
    #[error(
    "PFC-Dust-Migaloo: Unauthorized (action: {action:?}, expected: {expected:?}, actual: {actual:?})"
    )]
    Unauthorized {
        action: String,
        expected: String,
        actual: String,
    },
    #[error("PFC-Dust-Migaloo: Recursion Send_type {send_type:?} - {contract:?}")]
    Recursion { send_type: String, contract: String },

    #[error("PFC-Dust-Migaloo: ExecuteError Failed - {action:?}")]
    ExecuteError { action: String },
    #[error("PFC-Dust-Migaloo: denom not found - {denom:?}")]
    DenomNotFound { denom: String },
    #[error("PFC-Dust-Migaloo: Denom already exists - {name:?}")]
    DenomAlreadyThere { name: String },
    #[error("PFC-Dust-Migaloo: Invalid Denom - {denom:?}")]
    InvalidDenom { denom: String },
    #[error("PFC-Dust-Migaloo: No fees are defined. Add one before sending deposits")]
    NoFeesError {},
    #[error("PFC-Dust-Migaloo: Denom name must be unique")]
    DenomNotUnique {},

    #[error("PFC-Dust-Migaloo: Contract can't be migrated! {current_name:?} {current_version:?}")]
    MigrationError {
        current_name: String,
        current_version: String,
    },
    #[error("PFC-Dust-Migaloo: invalid reply {id} - {result}")]
    InvalidReply { id: u64, result: String },
    #[error("PFC-Dust-Migaloo: submessage fail - {error}")]
    SubMessageFail { error: String },
}
