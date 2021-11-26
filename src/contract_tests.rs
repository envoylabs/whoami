use crate::msg::{ExecuteMsg, Extension, Metadata, MintMsg};

use crate::{entry, ContractError, Cw721MetadataContract, InstantiateMsg};
use cosmwasm_std::DepsMut;

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
fn base_minting() {
    let mut deps = mock_dependencies();
    let contract = setup_contract(deps.as_mut());

    // init a plausible username
    let token_id = "jeffisthebest".to_string();
    let token_uri = "https://example.com/jeff-vader".to_string();

    let meta = Metadata {
        twitter_id: Some(String::from("@jeff-vader")),
        ..Metadata::default()
    };

    let mint_msg = ExecuteMsg::Mint(MintMsg {
        token_id: token_id.clone(),
        owner: String::from("jeff-vader"),
        token_uri: Some(token_uri.clone()),
        extension: meta.clone(),
    });

    // CHECK: random cannot mint with jeff as owner
    let random = mock_info("random", &[]);
    let err = entry::execute(deps.as_mut(), mock_env(), random, mint_msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // jeff can mint
    let allowed = mock_info(MINTER, &[]);
    let _ = entry::execute(deps.as_mut(), mock_env(), allowed, mint_msg).unwrap();

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
            token_uri: Some(token_uri.clone()),
            extension: meta,
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

    // CHECK: random can mint if sender && owner the same
    let john_q_rando = "random-guy";
    // another very plausible username imo
    let john_token_id = "johnisthebest".to_string();

    let john_q_rando_meta = Metadata {
        twitter_id: Some(String::from("@jeff-vader")),
        ..Metadata::default()
    };

    let john_q_rando_mint_msg = ExecuteMsg::Mint(MintMsg {
        token_id: john_token_id.clone(),
        owner: String::from(john_q_rando),
        token_uri: Some(token_uri.clone()),
        extension: john_q_rando_meta.clone(),
    });

    let not_jeff_minter = mock_info(john_q_rando, &[]);
    let _ = entry::execute(
        deps.as_mut(),
        mock_env(),
        not_jeff_minter,
        john_q_rando_mint_msg,
    )
    .unwrap();

    // ensure num tokens increases
    let count = contract.num_tokens(deps.as_ref()).unwrap();
    assert_eq!(2, count.count);

    // this nft info is correct
    let info = contract
        .nft_info(deps.as_ref(), john_token_id.clone())
        .unwrap();
    assert_eq!(
        info,
        NftInfoResponse::<Extension> {
            token_uri: Some(token_uri),
            extension: john_q_rando_meta,
        }
    );

    // owner info is correct
    let owner = contract
        .owner_of(deps.as_ref(), mock_env(), john_token_id.clone(), true)
        .unwrap();
    assert_eq!(
        owner,
        OwnerOfResponse {
            owner: String::from(john_q_rando),
            approvals: vec![],
        }
    );

    let meta2 = Metadata {
        twitter_id: Some(String::from("@jeff-vader-alt")),
        ..Metadata::default()
    };

    // CHECK: cannot mint same token_id again
    // even if minter & owner are the same
    let mint_msg2 = ExecuteMsg::Mint(MintMsg {
        token_id: token_id.clone(),
        owner: String::from("jeff-vader"),
        token_uri: None,
        extension: meta2.clone(),
    });

    let allowed = mock_info("jeff-vader", &[]);
    let err = entry::execute(deps.as_mut(), mock_env(), allowed, mint_msg2).unwrap_err();
    assert_eq!(err, ContractError::Claimed {});

    // list the token_ids
    let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
    assert_eq!(2, tokens.tokens.len());
    assert_eq!(vec![token_id.clone(), john_token_id.clone()], tokens.tokens);

    // CHECK: can mint second NFT
    // clearly there is an arms race by username proxy here
    let token_id_2 = "jeffisbetterthanjohn".to_string();
    let mint_msg3 = ExecuteMsg::Mint(MintMsg {
        token_id: token_id_2.clone(),
        owner: String::from("jeff-vader"),
        token_uri: None,
        extension: meta2,
    });

    let allowed = mock_info(MINTER, &[]);
    let _ = entry::execute(deps.as_mut(), mock_env(), allowed, mint_msg3).unwrap();

    // list the token_ids
    // four calls to mint, 3 tokens minted
    let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
    assert_eq!(3, tokens.tokens.len());
    assert_eq!(vec![token_id_2, token_id, john_token_id], tokens.tokens);
}

#[test]
fn use_metadata_extension() {
    let mut deps = mock_dependencies();
    let contract = Cw721MetadataContract::default();

    let info = mock_info(CREATOR, &[]);
    let init_msg = InstantiateMsg {
        name: "SpaceShips".to_string(),
        symbol: "SPACE".to_string(),
        minter: "jeff-addr".to_string(),
    };
    contract
        .instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg)
        .unwrap();

    // mock info contains sender &&
    // info.sender and owner need to be the same
    // that & MINTER do not need to be
    // as MINTER is the admin addr on the contract
    let token_id = "Enterprise";
    let mint_msg = MintMsg {
        token_id: token_id.to_string(),
        owner: CREATOR.to_string(),
        token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
        extension: Metadata {
            twitter_id: Some(String::from("@jeff-vader")),
            ..Metadata::default()
        },
    };
    let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
    entry::execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

    let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
    assert_eq!(res.token_uri, mint_msg.token_uri);
    assert_eq!(res.extension, mint_msg.extension);
}
