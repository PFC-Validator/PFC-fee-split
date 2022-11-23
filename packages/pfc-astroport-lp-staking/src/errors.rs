use cosmwasm_std::{DivideByZeroError, OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},
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
}
