pub mod state;

use cosmwasm_std::{DepsMut, Empty, Env, MessageInfo, Response};

use cw721_base::state::TokenInfo;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use cw721_base::{ContractError, InstantiateMsg, MintMsg, MinterResponse, QueryMsg};

use crate::state::USERNAMES;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

pub type Route = String;

// see: https://docs.opensea.io/docs/metadata-standards
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub twitter_id: Option<String>,
    pub discord_id: Option<String>,
    pub telegram_id: Option<String>,
    pub keybase_id: Option<String>,
    pub route: Option<Route>, // routes can be followed up the tree. if empty, then can assume it is ROOT
}

pub type Extension = Metadata;

pub type Cw721MetadataContract<'a> = cw721_base::Cw721Contract<'a, Extension, Empty>;
pub type ExecuteMsg = cw721_base::ExecuteMsg<Extension>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;

    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    // This is a simple type to let us handle empty extensions

    // This makes a conscious choice on the various generics used by the contract
    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        Cw721MetadataContract::default().instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        let tract = Cw721MetadataContract::default();
        match msg {
            ExecuteMsg::Mint(msg) => mint(tract, deps, env, info, msg),

            _ => tract
                .execute(deps, env, info, msg.into())
                .map_err(|err| err),
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        Cw721MetadataContract::default().query(deps, env, msg)
    }
}

pub fn get_path_parts(path: &str) -> Vec<&str> {
    let split = path.split("/");
    split.collect()
}

pub fn mint(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: MintMsg<Extension>,
) -> Result<Response, ContractError> {
    let minter = contract.minter.load(deps.storage)?;
    let address_trying_to_mint = info.sender.clone();

    if address_trying_to_mint != minter {
        return Err(ContractError::Unauthorized {});
    }

    // validate owner addr
    let owner_address = deps.api.addr_validate(&msg.owner)?;

    // username == token_id
    // validate username length. this, or to 128 bytes?
    let username = &msg.token_id;
    if username.chars().count() > 20 {
        return Err(ContractError::Unauthorized {});
    }

    // if we are trying to mint a subdomain,
    // check that the root is owned by the same address
    let path_parts = get_path_parts(&username);
    if path_parts.len() > 1 {
        let root_username = path_parts.clone().into_iter().nth(0).unwrap();
        // look up owner of root id
        let root_id_owner_addr = USERNAMES.load(deps.storage, &root_username)?;
        if address_trying_to_mint != root_id_owner_addr {
            return Err(ContractError::Unauthorized {});
        }
    }

    // create the token
    // this will fail if token_id (i.e. username)
    // is already claimed
    let token = TokenInfo {
        owner: owner_address.clone(),
        approvals: vec![],
        token_uri: msg.token_uri,
        extension: msg.extension,
    };
    contract
        .tokens
        .update(deps.storage, &msg.token_id, |old| match old {
            Some(_) => Err(ContractError::Claimed {}),
            None => Ok(token),
        })?;

    contract.increment_tokens(deps.storage)?;

    // set up secondary indexes
    USERNAMES.save(deps.storage, &username, &owner_address)?;

    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_attribute("minter", info.sender)
        .add_attribute("token_id", msg.token_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw721::{NftInfoResponse, OwnerOfResponse};

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cw721::Cw721Query;

    const CREATOR: &str = "creator";
    const MINTER: &str = "jeff-vader";
    const CONTRACT_NAME: &str = "Magic Power";
    const SYMBOL: &str = "MGK";

    fn setup_contract(deps: DepsMut<'_>) -> Cw721MetadataContract<'static> {
        let contract = Cw721MetadataContract::default();
        let msg = InstantiateMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            minter: String::from(MINTER),
        };
        let info = mock_info("creator", &[]);
        let res = contract.instantiate(deps, mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        contract
    }

    #[test]
    fn minting() {
        let mut deps = mock_dependencies();
        let contract = setup_contract(deps.as_mut());

        let token_id = "jeff".to_string();
        let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();

        let meta = Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        };

        let mint_msg = ExecuteMsg::Mint(MintMsg::<Extension> {
            token_id: token_id.clone(),
            owner: String::from("jeff-vader"),
            token_uri: Some(token_uri.clone()),
            extension: meta.clone(),
        });

        // random cannot mint
        let random = mock_info("random", &[]);
        let err = contract
            .execute(deps.as_mut(), mock_env(), random, mint_msg.clone())
            .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // minter can mint
        let allowed = mock_info(MINTER, &[]);
        let _ = contract
            .execute(deps.as_mut(), mock_env(), allowed, mint_msg)
            .unwrap();

        // ensure num tokens increases
        let count = contract.num_tokens(deps.as_ref()).unwrap();
        assert_eq!(1, count.count);

        // unknown nft returns error
        let _ = contract
            .nft_info(deps.as_ref(), "unknown".to_string())
            .unwrap_err();

        // this nft info is correct
        let info = contract.nft_info(deps.as_ref(), token_id.clone()).unwrap();
        assert_eq!(
            info,
            NftInfoResponse::<Extension> {
                token_uri: Some(token_uri),
                extension: meta.clone(),
            }
        );

        // owner info is correct
        let owner = contract
            .owner_of(deps.as_ref(), mock_env(), token_id.clone(), true)
            .unwrap();
        assert_eq!(
            owner,
            OwnerOfResponse {
                owner: String::from("jeff-vader"),
                approvals: vec![],
            }
        );

        let meta2 = Metadata {
            twitter_id: Some(String::from("@jeff-vader-alt")),
            ..Metadata::default()
        };

        // CHECK: cannot mint same token_id again
        let mint_msg2 = ExecuteMsg::Mint(MintMsg::<Extension> {
            token_id: token_id.clone(),
            owner: String::from("hercules"),
            token_uri: None,
            extension: meta2.clone(),
        });

        let allowed = mock_info(MINTER, &[]);
        let err = contract
            .execute(deps.as_mut(), mock_env(), allowed, mint_msg2)
            .unwrap_err();
        assert_eq!(err, ContractError::Claimed {});

        // list the token_ids
        let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
        assert_eq!(1, tokens.tokens.len());
        assert_eq!(vec![token_id.clone()], tokens.tokens);

        // CHECK: can mint second NFT
        let token_id_2 = "jeff-vader-2".to_string();
        let mint_msg3 = ExecuteMsg::Mint(MintMsg::<Extension> {
            token_id: token_id_2.clone(),
            owner: String::from("jeff alt"),
            token_uri: None,
            extension: meta2.clone(),
        });

        let allowed = mock_info(MINTER, &[]);
        let _ = contract
            .execute(deps.as_mut(), mock_env(), allowed, mint_msg3.clone())
            .unwrap();

        // list the token_ids
        let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
        assert_eq!(2, tokens.tokens.len());
        assert_eq!(vec![token_id.clone(), token_id_2.clone()], tokens.tokens);

        // CHECK: can mint nested NFT
        let token_id_nested = "jeff/vader".to_string();
        let mint_msg_nested = ExecuteMsg::Mint(MintMsg::<Extension> {
            token_id: token_id_nested.clone(),
            owner: String::from("jeff-vader"),
            token_uri: None,
            extension: meta2.clone(),
        });

        let allowed = mock_info(MINTER, &[]);
        let _ = contract
            .execute(deps.as_mut(), mock_env(), allowed, mint_msg_nested)
            .unwrap();

        // list the token_ids
        let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
        assert_eq!(3, tokens.tokens.len());
        assert_eq!(
            vec![
                token_id.clone(),
                token_id_2.clone(),
                token_id_nested.clone()
            ],
            tokens.tokens
        );

        // CHECK: cannot mint nested NFT if not the original root owner
        let token_id_nested = "jeff/vader/secret-plans".to_string();
        let mint_msg_nested_2 = ExecuteMsg::Mint(MintMsg::<Extension> {
            token_id: token_id_nested.clone(),
            owner: String::from("some-random-guy"),
            token_uri: None,
            extension: meta2.clone(),
        });

        let allowed = mock_info(MINTER, &[]);
        let nested_err = contract
            .execute(deps.as_mut(), mock_env(), allowed, mint_msg_nested_2)
            .unwrap_err();
        assert_eq!(nested_err, ContractError::Unauthorized {});

        // list the token_ids
        let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
        assert_eq!(3, tokens.tokens.len());
        assert_eq!(
            vec![
                token_id.clone(),
                token_id_2.clone(),
                token_id_nested.clone()
            ],
            tokens.tokens
        );
    }

    #[test]
    fn use_metadata_extension() {
        let mut deps = mock_dependencies();
        let contract = Cw721MetadataContract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
        };
        contract
            .instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg)
            .unwrap();

        let token_id = "Enterprise";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: "jeff-addr".to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Metadata {
                twitter_id: Some(String::from("@jeff-vader")),
                ..Metadata::default()
            },
        };
        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        contract
            .execute(deps.as_mut(), mock_env(), info, exec_msg)
            .unwrap();

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        assert_eq!(res.token_uri, mint_msg.token_uri);
        assert_eq!(res.extension, mint_msg.extension);
    }
}
