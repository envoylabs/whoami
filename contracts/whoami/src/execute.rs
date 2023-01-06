use crate::error::ContractError;
use cosmwasm_std::{ensure_eq, Binary, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use cw2::set_contract_version;
use cw721::Cw721ReceiveMsg;
use cw721_base::state::TokenInfo;
use cw_utils::{must_pay, one_coin};

use std::convert::TryInto;

use crate::msg::{
    ContractInfo, InstantiateMsg, Metadata, MintMsg, MintingFeesResponse, UpdateMetadataMsg,
    UpdateMintingFeesMsg,
};

use crate::query::get_paths_for_owner_and_token;
use crate::state::{
    CONTRACT_INFO, DID_CONTRACT_ADDRESS, DID_METHOD, MINTING_FEES_INFO, PRIMARY_ALIASES,
    USERNAME_LENGTH_CAP,
};
use crate::utils::{
    get_mint_fee, get_mint_response, get_number_of_owned_tokens, get_username_length, is_path,
    path_is_valid, pgp_pubkey_format_is_valid, username_is_valid, validate_did_method,
    validate_subdomain, verify_logo,
};
use crate::Cw721MetadataContract;

// version info for migration info
pub const CONTRACT_NAME: &str = "crates.io:whoami";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn execute_instantiate(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let info = ContractInfo {
        name: msg.name,
        symbol: msg.symbol,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;

    if let Some(ulc) = msg.username_length_cap {
        USERNAME_LENGTH_CAP.save(deps.storage, &ulc)?;
    }

    validate_did_method(msg.did_method.clone())?;
    DID_METHOD.save(deps.storage, &msg.did_method)?;

    // set to None initially
    DID_CONTRACT_ADDRESS.save(deps.storage, None)?;

    let minting_fees = MintingFeesResponse {
        native_denom: msg.native_denom,
        native_decimals: msg.native_decimals,
        token_cap: msg.token_cap,
        base_mint_fee: msg.base_mint_fee,
        burn_percentage: msg.burn_percentage,
        short_name_surcharge: msg.short_name_surcharge,
    };
    MINTING_FEES_INFO.save(deps.storage, &minting_fees)?;
    let admin_address = deps.api.addr_validate(&msg.admin_address)?;
    contract.minter.save(deps.storage, &admin_address)?;
    Ok(Response::default())
}

pub fn update_did_contract_address(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    env: Env,
    did_contract_address: String,
) -> Result<Response, ContractError> {
    let validated_address = deps.api.addr_validate(&did_contract_address)?;

    DID_CONTRACT_ADDRESS.save(deps.storage, validated_address.clone())?;

    let res = Response::new()
        .add_attribute("action", "update_did_contract_address")
        .add_attribute("new_did_contract_address", validated_address);
    Ok(res)
}

// update minting fees
pub fn update_minting_fees(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: UpdateMintingFeesMsg,
) -> Result<Response, ContractError> {
    let address_trying_to_update = info.sender;

    // look up contract admin
    let current_admin_address = contract.minter(deps.as_ref())?.minter;

    // check it's the admin of the contract updating
    ensure_eq!(
        current_admin_address,
        address_trying_to_update,
        ContractError::Unauthorized {}
    );

    // get current fees
    let minting_fees_info = MINTING_FEES_INFO.load(deps.storage)?;

    let minting_fees = MintingFeesResponse {
        // these two can't be updated
        native_denom: minting_fees_info.native_denom,
        native_decimals: minting_fees_info.native_decimals,
        // these can
        token_cap: msg.token_cap,
        base_mint_fee: msg.base_mint_fee,
        burn_percentage: msg.burn_percentage,
        short_name_surcharge: msg.short_name_surcharge,
    };

    // update
    MINTING_FEES_INFO.save(deps.storage, &minting_fees)?;

    let res = Response::new().add_attribute("action", "update_contract_minting_fees");
    Ok(res)
}

// the admin addr can update the cap on usernames length
pub fn set_username_length_cap(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_length: u32,
) -> Result<Response, ContractError> {
    let address_trying_to_update = info.sender;
    let current_admin_address = contract.minter(deps.as_ref())?.minter;

    // check it's the admin of the contract updating
    ensure_eq!(
        current_admin_address,
        address_trying_to_update,
        ContractError::Unauthorized {}
    );

    // init default
    let default_cap = 20;

    // can't decrease cap below previous or default
    let username_length_cap = USERNAME_LENGTH_CAP.may_load(deps.storage).unwrap();

    let cap = match username_length_cap {
        Some(ulc) => {
            if new_length <= ulc {
                ulc
            } else if new_length <= default_cap {
                default_cap
            } else {
                new_length
            }
        }
        None => {
            if new_length <= default_cap {
                default_cap
            } else {
                new_length
            }
        }
    };

    // set to new
    USERNAME_LENGTH_CAP.save(deps.storage, &cap)?;

    let res = Response::new()
        .add_attribute("action", "update_username_length_cap")
        .add_attribute("new_length_cap", Uint128::new(cap.into()));
    Ok(res)
}

// this actually updates the ADMIN address, but under the hood it is
// called minter by the contract.
// On the query side we actually just proxy to the existing Minter query
pub fn set_admin_address(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    admin_address: String,
) -> Result<Response, ContractError> {
    let address_trying_to_update = info.sender;
    let current_admin_address = contract.minter(deps.as_ref())?.minter;

    // check it's the admin of the contract updating
    ensure_eq!(
        current_admin_address,
        address_trying_to_update,
        ContractError::Unauthorized {}
    );

    // validate
    let validated_addr = deps.api.addr_validate(&admin_address)?;

    // update
    contract.minter.save(deps.storage, &validated_addr)?;

    let res = Response::new()
        .add_attribute("action", "update_contract_admin_address")
        .add_attribute("new_admin_address", validated_addr);
    Ok(res)
}

// boy oh boy this needs a refactor
pub fn mint(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: MintMsg,
) -> Result<Response, ContractError> {
    // any address can mint
    // sender of the execute
    let address_trying_to_mint = info.sender.clone();

    // can only mint NFTs belonging to yourself
    ensure_eq!(
        msg.owner,
        address_trying_to_mint,
        ContractError::Unauthorized {}
    );

    // validate any embedded logo
    if let Some(ref pfp_data) = msg.extension.image_data {
        verify_logo(pfp_data)?
    }

    // validate PGP pubkey format
    if let Some(ref pgp_public_key) = msg.extension.pgp_public_key {
        if !pgp_pubkey_format_is_valid(pgp_public_key) {
            return Err(ContractError::InvalidPgpPublicKey {});
        }
    }

    // get minting fees and minter (i.e. admin)
    let minting_fees = MINTING_FEES_INFO.load(deps.storage)?;
    let minter = contract.minter(deps.as_ref())?.minter;
    let admin_address = deps.api.addr_validate(&minter)?;

    // check if trying to mint too many
    // who can need more than 20?
    let default_limit: usize = 20;
    let number_of_tokens_owned = get_number_of_owned_tokens(
        &contract,
        &deps,
        address_trying_to_mint.clone(),
        default_limit,
    )?;

    // error out if we exceed configured cap or we already
    // have the default max
    match minting_fees.token_cap {
        Some(tc) => {
            if number_of_tokens_owned >= tc.try_into().unwrap() {
                return Err(ContractError::TokenCapExceeded {});
            }
        }
        None => {
            if number_of_tokens_owned == default_limit {
                return Err(ContractError::TokenCapExceeded {});
            }
        }
    }

    // validate owner addr
    let owner_address = deps.api.addr_validate(&msg.owner)?;

    // username == token_id
    // normalize it to lowercase
    let username = &msg.token_id.to_lowercase();
    if !username_is_valid(deps.as_ref(), username) {
        return Err(ContractError::TokenNameInvalid {});
    }

    // if parent_token_id is set,
    // this is a subdomain
    // we also check for cycles
    if let Some(ref parent_token_id) = msg.extension.parent_token_id {
        if parent_token_id == username || is_path(parent_token_id) {
            return Err(ContractError::CycleDetected {});
        } else {
            validate_subdomain(
                &contract,
                &deps,
                parent_token_id.to_string(),
                address_trying_to_mint.clone(),
            )?;
        }
    }

    // work out what fees are owed
    let fee = get_mint_fee(minting_fees.clone(), get_username_length(username));
    // error out if this fee isn't covered in the msg
    if fee.is_some() {
        must_pay(&info, &minting_fees.native_denom)?;

        // ensure atomicity
        let coin = one_coin(&info)?;
        if let Some(fee_amount) = fee {
            if coin.amount < fee_amount {
                return Err(ContractError::InsufficientFunds {});
            }
        }
    };

    // create the token
    // this will fail if token_id (i.e. username)
    // is already claimed
    let token = TokenInfo {
        owner: owner_address,
        approvals: vec![],
        token_uri: msg.token_uri,
        extension: msg.extension,
    };
    contract
        .tokens
        .update(deps.storage, username, |old| match old {
            Some(_) => Err(ContractError::Claimed {}),
            None => Ok(token),
        })?;

    contract.increment_tokens(deps.storage)?;

    // if there is a fee, add a bank msg to send to the admin_address
    let res = get_mint_response(
        admin_address,
        address_trying_to_mint,
        minting_fees.native_denom,
        fee,
        minting_fees.burn_percentage,
        msg.token_id,
    );
    Ok(res)
}

// mint a PATH
// essentially what we call a reified subdomain/namespace
// where the whole slug is a single item
// paths are different from names
// they are free to mint, and have no cap
pub fn mint_path(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: MintMsg,
) -> Result<Response, ContractError> {
    // any address can mint
    // sender of the execute
    let address_trying_to_mint = info.sender;

    // can only mint NFTs belonging to yourself
    ensure_eq!(
        msg.owner,
        address_trying_to_mint,
        ContractError::Unauthorized {}
    );

    // validate any embedded logo or image
    if let Some(ref pfp_data) = msg.extension.image_data {
        verify_logo(pfp_data)?
    }

    // validate PGP public key format
    // or return an error
    if let Some(ref pgp_public_key) = msg.extension.pgp_public_key {
        if !pgp_pubkey_format_is_valid(pgp_public_key) {
            return Err(ContractError::InvalidPgpPublicKey {});
        }
    }

    // validate owner addr
    let owner_address = deps.api.addr_validate(&msg.owner)?;

    // path == token_id
    // normalize it to lowercase
    let path = &msg.token_id.to_lowercase();

    // if parent_token_id is set,
    // this is a path (if not, it's invalid)
    // we also check for cycles
    // we also check that parent token isn't repeated in the path
    if let Some(ref parent_token_id) = msg.extension.parent_token_id {
        if parent_token_id == path {
            Err(ContractError::CycleDetected {})
        } else {
            // first we validate path
            if !path_is_valid(path, parent_token_id) {
                return Err(ContractError::TokenNameInvalid {});
            }

            // then its hierarchy
            validate_subdomain(
                &contract,
                &deps,
                parent_token_id.to_string(),
                address_trying_to_mint.clone(),
            )?;

            // okay, it's valid, prepend it with parent and start the show
            let full_path = format!("{}.{}", parent_token_id, path);

            // create the token
            // this will fail if claimed
            let token = TokenInfo {
                owner: owner_address,
                approvals: vec![],
                token_uri: msg.token_uri,
                extension: msg.extension,
            };
            contract
                .tokens
                .update(deps.storage, &full_path, |old| match old {
                    Some(_) => Err(ContractError::Claimed {}),
                    None => Ok(token),
                })?;

            contract.increment_tokens(deps.storage)?;

            let res = Response::new()
                .add_attribute("action", "mint")
                .add_attribute("minter", address_trying_to_mint)
                .add_attribute("token_id", full_path);
            Ok(res)
        }
    } else {
        Err(ContractError::ParentNotFound {})
    }
}

// updates the metadata on an NFT
// only accessible to the NFT owner
// note that the parent_token_id field
// is immutable and cannot be updated
pub fn update_metadata(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: UpdateMetadataMsg,
) -> Result<Response, ContractError> {
    let address_trying_to_update = info.sender.clone();
    let token_id = msg.token_id.clone();
    let username_nft = contract.tokens.load(deps.storage, &token_id)?;

    let username_owner = username_nft.owner.clone();

    // this is immutable
    let existing_parent_id = username_nft.extension.parent_token_id.clone();

    // check it's the owner of the NFT updating meta
    ensure_eq!(
        username_owner,
        address_trying_to_update,
        ContractError::Unauthorized {}
    );

    // validate any embedded logo
    if let Some(ref pfp_data) = msg.metadata.image_data {
        verify_logo(pfp_data)?
    }

    // arrrrre you ready to rrrrrumb-
    // rrredefine some metadata?
    contract
        .tokens
        .update(deps.storage, &token_id, |token| -> StdResult<_> {
            match token {
                Some(mut nft) => {
                    nft.extension = msg.metadata;
                    nft.extension.parent_token_id = existing_parent_id;
                    Ok(nft)
                }
                None => Ok(username_nft),
            }
        })?;

    Ok(Response::new()
        .add_attribute("action", "update_metadata")
        .add_attribute("owner", info.sender)
        .add_attribute("token_id", token_id))
}

// look up token_id
// if it is owned by sender,
// then set mapping of sender -> token_id
pub fn update_primary_alias(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let address_trying_to_update = info.sender;
    let username_nft = contract.tokens.load(deps.storage, &token_id)?;

    let username_owner = username_nft.owner;

    // check it's the owner of the NFT updating the mapping
    ensure_eq!(
        username_owner,
        address_trying_to_update,
        ContractError::Unauthorized {}
    );

    // always overwrite
    PRIMARY_ALIASES.save(deps.storage, &address_trying_to_update, &token_id)?;

    let res = Response::new()
        .add_attribute("action", "update_preferred_alias")
        .add_attribute("address", address_trying_to_update)
        .add_attribute("username", token_id);
    Ok(res)
}

//
// --- we override these purely so we can clear any preferred aliases and sub paths on transfer or burn
//

pub fn clear_alias_if_primary(deps: DepsMut, token_id: String) -> Result<(), ContractError> {
    let contract = Cw721MetadataContract::default();
    let username_nft = contract.tokens.load(deps.storage, &token_id)?;
    let primary_alias = PRIMARY_ALIASES.may_load(deps.storage, &username_nft.owner)?;
    if let Some(alias) = primary_alias {
        if alias == token_id {
            PRIMARY_ALIASES.remove(deps.storage, &username_nft.owner);
        }
    }
    Ok(())
}

// this function clears metadata
// for situations like transfer and send
// to enable web of trust stuff
// and make sure stale meta doesn't persist after send/transfer
pub fn clear_metadata(deps: DepsMut, token_id: String) -> Result<(), ContractError> {
    let contract = Cw721MetadataContract::default();
    let username_nft = contract.tokens.load(deps.storage, &token_id)?;
    contract
        .tokens
        .update(deps.storage, &token_id, |token| -> StdResult<_> {
            match token {
                Some(mut nft) => {
                    nft.extension = Metadata {
                        ..Metadata::default()
                    };
                    Ok(nft)
                }
                None => Ok(username_nft),
            }
        })?;
    Ok(())
}

// this function burns all paths
// that sit under a token
pub fn burn_paths(deps: DepsMut, token_id: String) -> Result<(), ContractError> {
    let contract = Cw721MetadataContract::default();
    let base_nft = contract.tokens.load(deps.storage, &token_id)?;
    let owner_addr = base_nft.owner;

    let paths =
        get_paths_for_owner_and_token(deps.as_ref(), owner_addr.to_string(), token_id, None, None)?
            .tokens;

    for path_id in paths {
        contract.tokens.remove(deps.storage, &path_id)?;
        contract.decrement_tokens(deps.storage)?;
    }

    Ok(())
}

pub fn transfer_nft(
    contract: Cw721MetadataContract,
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    token_id: String,
) -> Result<Response, ContractError> {
    // check permissions before proceeding
    let token = contract.tokens.load(deps.storage, &token_id)?;
    contract.check_can_send(deps.as_ref(), &env, &info, &token)?;

    // clear aliases before transfer iif it is the one being xfrd
    clear_alias_if_primary(deps.branch(), token_id.to_string())?;

    // blank meta before xfer
    clear_metadata(deps.branch(), token_id.to_string())?;

    // clear paths
    burn_paths(deps.branch(), token_id.to_string())?;

    contract._transfer_nft(deps, &env, &info, &recipient, &token_id)?;

    Ok(Response::new()
        .add_attribute("action", "transfer_nft")
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", recipient)
        .add_attribute("token_id", token_id))
}

pub fn send_nft(
    contract: Cw721MetadataContract,
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    receiving_contract: String,
    token_id: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    // check permissions before proceeding
    let token = contract.tokens.load(deps.storage, &token_id)?;
    contract.check_can_send(deps.as_ref(), &env, &info, &token)?;

    // clear aliases before send iif it is the one being sent
    clear_alias_if_primary(deps.branch(), token_id.to_string())?;

    // blank meta before send
    clear_metadata(deps.branch(), token_id.to_string())?;

    // clear paths
    burn_paths(deps.branch(), token_id.to_string())?;

    // Transfer token
    contract._transfer_nft(deps, &env, &info, &receiving_contract, &token_id)?;

    let send = Cw721ReceiveMsg {
        sender: info.sender.to_string(),
        token_id: token_id.clone(),
        msg,
    };

    // Send message
    Ok(Response::new()
        .add_message(send.into_cosmos_msg(receiving_contract.clone())?)
        .add_attribute("action", "send_nft")
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", receiving_contract)
        .add_attribute("token_id", token_id))
}

pub fn burn(
    contract: Cw721MetadataContract,
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let token = contract.tokens.load(deps.storage, &token_id)?;
    contract.check_can_send(deps.as_ref(), &env, &info, &token)?;

    // clear aliases before delete iif it is the one being burned
    clear_alias_if_primary(deps.branch(), token_id.to_string())?;

    // clear paths
    burn_paths(deps.branch(), token_id.to_string())?;

    contract.tokens.remove(deps.storage, &token_id)?;
    contract.decrement_tokens(deps.storage)?;

    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("sender", info.sender)
        .add_attribute("token_id", token_id))
}
