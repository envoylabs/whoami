use crate::state::DID_DOCUMENTS;
use crate::ContractError;
use cosmwasm_std::{ensure_eq, Deps};
use whoami_did::msg::DidDocument;

fn throw_error_if_did_exists(deps: Deps, did_id: String) -> Result<(), ContractError> {
    // if this doesn't return None then the DID exists
    // and thus the call should fail
    let did_exists: Option<DidDocument> = DID_DOCUMENTS.may_load(deps.storage, did_id)?;
    ensure_eq!(did_exists, None, ContractError::Unauthorized {});
    Ok(())
}

fn throw_error_if_sender_is_not_controller(
    deps: Deps,
    sender: String,
    did_id: String,
) -> Result<(), ContractError> {
}

fn throw_error_if_did_format_incorrect(did_id: String) -> Result<(), ContractError> {}
