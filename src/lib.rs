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

pub type Extension = Option<Metadata>;

pub type Cw721MetadataContract<'a> = cw721_base::Cw721Contract<'a, Extension, Empty>;
pub type ExecuteMsg = cw721_base::ExecuteMsg<Extension>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;

    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

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

            _ => tract.execute(deps, env, info, msg).map_err(|err| err),
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        Cw721MetadataContract::default().query(deps, env, msg)
    }
}

pub fn get_path_parts(path: &str) -> Vec<&str> {
    let split = path.split('/');
    split.collect()
}

pub fn get_root_token_id(path: &str) -> String {
    let path_parts = get_path_parts(&path);
    let root_token_id = path_parts.clone().into_iter().next().unwrap();
    // look up owner of root id
    root_token_id.to_string()
}

// username = path = token_id
// i.e. could be the-frey
// could be products/garden/1
pub fn mint(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: MintMsg<Extension>,
) -> Result<Response, ContractError> {
    let minter = contract.minter.load(deps.storage)?;
    // sender of the execute
    let address_trying_to_mint = info.sender.clone();

    if address_trying_to_mint != minter {
        return Err(ContractError::Unauthorized {});
    }

    // validate owner addr
    let owner_address = deps.api.addr_validate(&msg.owner)?;

    // username == token_id
    // validate username length. this, or to some number of bytes?
    let username = &msg.token_id;
    if username.chars().count() > 64 {
        return Err(ContractError::Unauthorized {});
    }

    // if we are trying to mint a subdomain,
    // check that the root is owned by the same address
    // look up owner Addr of root id by username (i.e. path)
    let path_parts = get_path_parts(&username);

    if path_parts.len() > 1 {
        let root_token_id = get_root_token_id(&username);
        let root_part_owner_addr = USERNAMES.load(deps.storage, &root_token_id)?;

        if address_trying_to_mint.ne(&root_part_owner_addr) {
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
        .update(deps.storage, &username, |old| match old {
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
    const CONTRACT_NAME: &str = "whoami";
    const SYMBOL: &str = "WHO";

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
    fn path_parts() {
        let not_nested = "jeff";
        let path = get_path_parts(not_nested);

        assert_eq!(path, vec!["jeff"]);
        assert_eq!(path.len(), 1);

        let nested = "jeff/vader/secret-plans";
        let path_parts = get_path_parts(nested);

        assert_eq!(path_parts, vec!["jeff", "vader", "secret-plans"]);
        assert_eq!(path_parts.len(), 3);
    }

    #[test]
    fn base_minting() {
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
            extension: Some(meta.clone()),
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
                extension: Some(meta),
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
            extension: Some(meta2),
        });

        let allowed = mock_info(MINTER, &[]);
        let err = contract
            .execute(deps.as_mut(), mock_env(), allowed, mint_msg2)
            .unwrap_err();
        assert_eq!(err, ContractError::Claimed {});

        // list the token_ids
        let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
        assert_eq!(1, tokens.tokens.len());
        assert_eq!(vec![token_id], tokens.tokens);
    }

    #[test]
    fn namespace_minting() {
        let jeff_address = String::from("jeff-vader");
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
            owner: jeff_address.clone(),
            token_uri: Some(token_uri.clone()),
            extension: Some(meta.clone()),
        });

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
                extension: Some(meta),
            }
        );

        let meta2 = Metadata {
            twitter_id: Some(String::from("@jeff-vader-alt")),
            ..Metadata::default()
        };

        // CHECK: can mint second NFT
        let token_id_2 = "jeff-vader-2".to_string();
        let mint_msg3 = ExecuteMsg::Mint(MintMsg::<Extension> {
            token_id: token_id_2.clone(),
            owner: String::from("jeff alt"),
            token_uri: None,
            extension: Some(meta2.clone()),
        });

        let allowed = mock_info(MINTER, &[]);
        let _ = contract
            .execute(deps.as_mut(), mock_env(), allowed, mint_msg3)
            .unwrap();

        // list the token_ids
        let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
        assert_eq!(2, tokens.tokens.len());
        assert_eq!(vec![token_id.clone(), token_id_2.clone()], tokens.tokens);

        // CHECK: can mint nested NFT
        let token_id_nested = "jeff/vader".to_string();
        let mint_msg_nested = ExecuteMsg::Mint(MintMsg::<Extension> {
            token_id: token_id_nested.clone(),
            owner: jeff_address.clone(),
            token_uri: None,
            extension: Some(meta2.clone()),
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
        let not_allowed_to_mint_nested = String::from("some-random-guy"); // i.e. not jeff_address
        let token_id_nested_bad = "jeff/vader/secret-plans".to_string();
        let mint_msg_nested_2 = ExecuteMsg::Mint(MintMsg::<Extension> {
            token_id: token_id_nested_bad,
            owner: not_allowed_to_mint_nested.clone(),
            token_uri: None,
            extension: Some(meta2),
        });

        let allowed = mock_info(MINTER, &[]);

        assert_ne!(not_allowed_to_mint_nested, jeff_address);

        let nested_err = contract
            .execute(deps.as_mut(), mock_env(), allowed, mint_msg_nested_2)
            .unwrap_err();
        assert_eq!(nested_err, ContractError::Unauthorized {});

        // list the token_ids - should still be the same as it was before, as we didn't mint the last token.
        let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
        assert_eq!(3, tokens.tokens.len());
        assert_eq!(vec![token_id, token_id_2, token_id_nested], tokens.tokens);
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
            extension: Some(Metadata {
                twitter_id: Some(String::from("@jeff-vader")),
                ..Metadata::default()
            }),
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
