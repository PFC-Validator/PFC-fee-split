use cosmwasm_std::{Coin, OverflowError, StdError};
//use protobuf::ProtobufError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("PFC-FeeSplit: StdError:{0}")]
    Std(#[from] StdError),

    #[error("PFC-FeeSplit: Overflow:{0}")]
    Overflow(#[from] OverflowError),

    #[error(
    "PFC-FeeSplit: Unauthorized (action: {action:?}, expected: {expected:?}, actual: {actual:?})"
    )]
    Unauthorized {
        action: String,
        expected: String,
        actual: String,
    },
    #[error("PFC-FeeSplit: Recursion Send_type {send_type:?} - {contract:?}")]
    Recursion { send_type: String, contract: String },
    #[error(transparent)]
    AdminError(#[from] cw_controllers::AdminError),

    #[error("PFC-FeeSplit: ExecuteError Failed - {action:?}")]
    ExecuteError { action: String },
    #[error("PFC-FeeSplit: Fee not found - {name:?}")]
    AllocationNotFound { name: String },
    #[error("PFC-FeeSplit: Fee already exists - {name:?}")]
    FeeAlreadyThere { name: String },
    #[error("PFC-FeeSplit: Invalid Coin - {coin:?}")]
    InvalidCoin { coin: Coin },
    #[error("PFC-FeeSplit: No fees are defined. Add one before sending deposits")]
    NoFeesError {},
    #[error("PFC-FeeSplit: Fund Allocation name must be unique")]
    FundAllocationNotUnique {},

    #[error("PFC-FeeSplit: Allocation has to be greater than zero")]
    AllocationZero {},

    #[error("PFC-FeeSplit: Contract can't be migrated! {current_name:?} {current_version:?}")]
    MigrationError {
        current_name: String,
        current_version: String,
    },
    #[error("PFC-FeeSplit: Reconcile should not be sent funds")]
    ReconcileWithFunds {},
}
