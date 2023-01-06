#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use whoami_did::msg::{
    DidArgs, DidDocument, DidDocumentResponse, DidExecuteMsg, DidQueryMsg, Service,
};

use crate::error::ContractError;
use crate::msg::InstantiateMsg;
use crate::state::{Config, CONFIG, DID_DOCUMENTS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:whoami-did";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DID_CONTEXT: &str = "https://www.w3.org/ns/did/v1";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if let Some(cc) = msg.controller_contract {
    } else {
    }

    let config = match msg.controller_contract {
        Some(cc) => {
            let validated_cc = deps.api.addr_validate(cc)?;
            Config {
                did_method: msg.did_method,
                controller_contract: Some(validated_cc),
            };
        }
        None => {
            Config {
                did_method: msg.did_method,
                controller_contract: None,
            };
        }
    };

    let config = Config {
        did_method: msg.did_method, // "minerva"
        controller_contract: msg.controller_contract,
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
        DidExecuteMsg::ProxyCreate { create_msg } => {
            execute_proxy_create(deps, env, info, create_msg)
        }
        DidExecuteMsg::Create { id } => create_did_document(deps, env, info, id, creator_address),
        DidExecuteMsg::Update { id } => update_did_document(deps, env, info, id),
        DidExecuteMsg::AddService { id, service } => {
            add_service_to_did_doc(deps, env, info, id, service)
        }
        DidExecuteMsg::DeleteService { id, service_id } => {
            delete_service_from_did_doc(deps, env, info, id, service_id)
        }
        DidExecuteMsg::Delete { id } => delete_did_document(deps, env, info, id),
    }
}

// unpack a wrapped create call
// this will error if no creator contract is set
fn execute_proxy_create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapped: CreateDidMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let controller_contract = config.controller_contract;

    if let Some(cc) = controller_contract {
    } else {
        return Err(ContractError::Unauthorized {});
    }
}

fn create_did_document(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    id: String,
    creator_address: String,
    did_meta: Option<DidArgs>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let did = format!("did:{}:{}", config.did_method, id);
    // todo: check if DID already exists
    // todo: check if the given id is in correct format. should not have a did prefix
    // todo: add info.sender address as controller
    // todo: get public key details and populate BlockchainAccountId struct

    // todo: validate and generate https://github.com/decentralized-identity/did-key.rs

    let did_doc = DidDocument {
        context: DID_CONTEXT.to_string(),
        id: did.clone(),
        controller: vec![creator_address],
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

fn add_service_to_did_doc(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _id: String,
    _service: Service,
) -> Result<Response, ContractError> {
    // todo : check if DID exists
    // todo : check if info.sender is a controller of did
    // todo : check if service with that id already exists
    // todo : push the service into the did document
    unimplemented!()
}

fn delete_service_from_did_doc(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _id: String,
    _service_id: String,
) -> Result<Response, ContractError> {
    // todo : check if DID exists
    // todo : check if info.sender is a controller of did
    // todo : check if service with that id exists
    // todo : delete the service from the did document
    unimplemented!()
}

fn update_did_document(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _id: String,
) -> Result<Response, ContractError> {
    // todo : check if DID exists
    // todo : check if info.sender is a controller of did
    // perform validity check on all did fields
    unimplemented!()
}

fn delete_did_document(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _id: String,
) -> Result<Response, ContractError> {
    // todo : check if DID exists
    // todo : check if info.sender is a controller of did
    // todo : preserve the did.id and purge everything else
    // todo : add the did to revocation list? we don't have a revocation list yet.
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
