use cosmwasm_std::StdError;
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Base(#[from] cw721_base::ContractError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("Claimed")]
    Claimed {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("No Links Permitted for Embedded Field")]
    NoLinksPermitted {},
}
