#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
// use cw2::set_contract_version;
use whoami_did::msg::{DidDocument, DidDocumentResponse, DidExecuteMsg, DidQueryMsg};

use crate::error::ContractError;
use crate::msg::InstantiateMsg;
use crate::state::{Config, CONFIG, DID_DOCUMENTS};

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
        DidExecuteMsg::Create { id } => create_did_document(deps, env, info, id),
        DidExecuteMsg::Update { id } => update_did_document(deps, env, info, id),
        DidExecuteMsg::Delete { id } => delete_did_document(deps, env, info, id),
    }
}

fn create_did_document(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
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

fn update_did_document(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _id: String,
) -> Result<Response, ContractError> {
    unimplemented!()
}

fn delete_did_document(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _id: String,
) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: DidQueryMsg) -> StdResult<Binary> {
    match msg {
        DidQueryMsg::Resolve { id } => resolve(id),
        DidQueryMsg::Read { did } => read_did_document(deps, did),
    }
}

fn read_did_document(deps: Deps, did: String) -> StdResult<Binary> {
    let did_document = DID_DOCUMENTS.may_load(deps.storage, did)?;
    to_binary(&DidDocumentResponse { did_document })
}

fn resolve(_id: String) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::{execute, instantiate};
    use crate::{contract::query, msg::InstantiateMsg};
    use cosmwasm_std::{
        attr, from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
    };
    use whoami_did::msg::{DidDocumentResponse, DidExecuteMsg, DidQueryMsg};

    // Two fake addresses we will use to mock_info
    pub const ADDR1: &str = "addr1";

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(ADDR1, &vec![]);

        let msg = InstantiateMsg {
            did_method: "minerva".to_string(),
        };
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![attr("action", "instantiate"), attr("did_method", "minerva")]
        )
    }

    #[test]
    fn test_create_did_document() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            did_method: "test".to_string(),
        };
        let info = mock_info(ADDR1, &vec![]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info(ADDR1, &vec![]);
        let msg = DidExecuteMsg::Create {
            id: "id".to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![attr("action", "create_did"), attr("did", "did:test:id")]
        )
    }

    #[test]
    fn test_read_did_document() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            did_method: "test".to_string(),
        };
        let info = mock_info(ADDR1, &vec![]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info(ADDR1, &vec![]);
        let msg = DidExecuteMsg::Create {
            id: "id".to_string(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // querying the did we just created. should exist
        let msg = DidQueryMsg::Read {
            did: "did:test:id".to_string(),
        };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let did_doc_res: DidDocumentResponse = from_binary(&res).unwrap();
        assert!(did_doc_res.did_document.is_some());

        // querying a did we did not create. should not exist
        let msg = DidQueryMsg::Read {
            did: "did:test:does_not_exist".to_string(),
        };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let did_doc_res: DidDocumentResponse = from_binary(&res).unwrap();
        assert!(did_doc_res.did_document.is_none());
    }
}
