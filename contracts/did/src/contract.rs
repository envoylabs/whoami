#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
// use cw2::set_contract_version;
use whoami_did::msg::{DidExecuteMsg, DidQueryMsg, DidDocument};

use crate::error::ContractError;
use crate::msg::{InstantiateMsg};
use crate::state::{DID_DOCUMENTS, Config, CONFIG};


/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:did";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/
const DID_CONTEXT: &str = "https://www.w3.org/ns/did/v1";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        did_method: msg.did_method, //"minerva"
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
    .add_attribute("action", "instantiate")
    .add_attribute("did_method", config.did_method))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: DidExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        DidExecuteMsg::Create { id } => {create_did_document(deps, env, info, id)},
        DidExecuteMsg::Update { id } => {unimplemented!()},
        DidExecuteMsg::Delete { id } => {unimplemented!()}
    }
}

fn create_did_document(deps: DepsMut, _env: Env, _info: MessageInfo, id: String) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let did = format!("did:{}:{}", config.did_method, id);
    // todo: check if DID already exists
    // todo: add info.sender address as controller
    
    let did_doc = DidDocument {
        context: DID_CONTEXT.to_string(),
        id: did.clone(),
        controller: vec![],
        verification_method: None,
        service: None,
        assertion_method: None,
        key_agreement: None,
        capability_invocation: None,
        capability_delegation: None,
    };
    DID_DOCUMENTS.save(deps.storage, did, &did_doc)?;
    Ok(Response::new()
    .add_attribute("action", "create_did")
    .add_attribute("did", did_doc.id))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: DidQueryMsg) -> StdResult<Binary> {
    match msg {
        DidQueryMsg::Read { did } => {unimplemented!()},
        DidQueryMsg::Resolve { id } => {read_did_document(deps, env, id)}
    }
}

fn read_did_document(deps: Deps, env: Env, id: String) -> StdResult<Binary> {
    todo!()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, attr};
    use whoami_did::msg::DidExecuteMsg;
    use crate::msg::{InstantiateMsg};
    use super::{instantiate, execute};

    // Two fake addresses we will use to mock_info
    pub const ADDR1: &str = "addr1";

    #[test]
    fn test_instantiate() {
        // Mock the dependencies, must be mutable so we can pass it as a mutable, empty vector means our contract has no balance
        let mut deps = mock_dependencies();
        // Mock the contract environment, contains the block info, contract address, etc.
        let env = mock_env();
        // Mock the message info, ADDR1 will be the sender, the empty vec means we sent no funds.
        let info = mock_info(ADDR1, &vec![]);

        let msg = InstantiateMsg { did_method: "minerva".to_string() };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![attr("action", "instantiate"), attr("did_method", "minerva")]
        )
    }
    #[test]
    fn create_did_document() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { did_method: "test".to_string() };
        let info = mock_info(ADDR1, &vec![]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info(ADDR1, &vec![]);
        let msg = DidExecuteMsg::Create { id: "id".to_string() };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![attr("action", "create_did"), attr("did", "did:test:id")]
        )
    }
}
