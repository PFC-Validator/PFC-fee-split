use cosmwasm_std::{CheckedFromRatioError, DivideByZeroError, OverflowError, StdError};
use cw_controllers::AdminError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    AdminError(#[from] AdminError),

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),
    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Rewards have already been calculated on this token. Can not change")]
    RewardsPresent {},

    #[error("None Bonded")]
    NoneBonded {},

    #[error("Asset mismatch")]
    AssetMismatch {},

    #[error("Not found")]
    NotFound {},

    #[error("Exceed limit")]
    ExceedLimit {},

    #[error("Already exists")]
    AlreadyExists {},
    #[error("Contract can't be migrated! {current_name:?} {current_version:?}")]
    MigrationError {
        current_name: String,
        current_version: String,
    },
}
