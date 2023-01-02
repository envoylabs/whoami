#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
// use cw2::set_contract_version;
use whoami_did::msg::{DidExecuteMsg, DidQueryMsg};

use crate::error::ContractError;
use crate::msg::{InstantiateMsg};


/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:did";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: DidExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        DidExecuteMsg::Create { id } => {unimplemented!()},
        DidExecuteMsg::Update { id } => {unimplemented!()},
        DidExecuteMsg::Delete { id } => {unimplemented!()}
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: DidQueryMsg) -> StdResult<Binary> {
    match msg {
        DidQueryMsg::Read { id } => {unimplemented!()},
        DidQueryMsg::Resolve { id } => {unimplemented!()}
    }
}

#[cfg(test)]
mod tests {}
