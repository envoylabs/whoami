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

    #[error("Insufficient Funds")]
    InsufficientFunds {},

    #[error("Claimed")]
    Claimed {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Token Cap Exceeded")]
    TokenCapExceeded {},

    #[error("Token Name Invalid")]
    TokenNameInvalid {},

    #[error("Required Parent Not Found")]
    ParentNotFound {},

    #[error("Unauthorized")]
    CycleDetected {},

    #[error("No Links Permitted for Embedded Field")]
    NoLinksPermitted {},

    #[error("Format is incorrect for PGP Public Key")]
    InvalidPgpPublicKey,
}
