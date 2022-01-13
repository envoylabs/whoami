use crate::error::ContractError;
use cosmwasm_std::{Binary, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use cw721::Cw721ReceiveMsg;
use cw721_base::state::TokenInfo;
use cw_utils::must_pay;

use std::convert::TryInto;

use crate::msg::{
    ContractInfo, InstantiateMsg, Metadata, MintMsg, MintingFeesResponse, UpdateMetadataMsg,
    UpdateMintingFeesMsg,
};

use crate::state::{CONTRACT_INFO, MINTING_FEES_INFO, PRIMARY_ALIASES};
use crate::utils::{
    get_mint_fee, get_mint_response, get_number_of_owned_tokens, get_username_length,
    username_is_valid, validate_subdomain, verify_logo,
};
use crate::Cw721MetadataContract;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:whoami";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn execute_instantiate(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let info = ContractInfo {
        name: msg.name,
        symbol: msg.symbol,
    };
    CONTRACT_INFO.save(deps.storage, &info)?;

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
    if current_admin_address != address_trying_to_update {
        return Err(ContractError::Unauthorized {});
    }

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
    if current_admin_address != address_trying_to_update {
        return Err(ContractError::Unauthorized {});
    }

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
    if address_trying_to_mint != msg.owner {
        return Err(ContractError::Unauthorized {});
    }

    // validate any embedded logo
    if let Some(ref pfp_data) = msg.extension.image_data {
        verify_logo(&pfp_data)?
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
    if !username_is_valid(username) {
        return Err(ContractError::TokenNameInvalid {});
    }

    // if parent_token_id is set,
    // this is a subdomain
    // we also check for cycles
    if let Some(ref parent_token_id) = msg.extension.parent_token_id {
        if parent_token_id == username {
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
        .update(deps.storage, &username, |old| match old {
            Some(_) => Err(ContractError::Claimed {}),
            None => Ok(token),
        })?;

    contract.increment_tokens(deps.storage)?;

    // if there is a fee, add a bank msg to send to the admin_address
    // TODO - implement burn of 50%
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

// updates the metadata on an NFT
// only accessible to the NFT owner
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

    // check it's the owner of the NFT updating meta
    if username_owner != address_trying_to_update {
        return Err(ContractError::Unauthorized {});
    }

    // validate any embedded logo
    if let Some(ref pfp_data) = msg.metadata.image_data {
        verify_logo(&pfp_data)?
    }

    // arrrrre you ready to rrrrrumb-
    // rrredefine some metadata?
    contract
        .tokens
        .update(deps.storage, &token_id, |token| -> StdResult<_> {
            match token {
                Some(mut nft) => {
                    nft.extension = msg.metadata;
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
    if username_owner != address_trying_to_update {
        return Err(ContractError::Unauthorized {});
    }

    // always overwrite
    PRIMARY_ALIASES.save(deps.storage, &address_trying_to_update, &token_id)?;

    let res = Response::new()
        .add_attribute("action", "update_preferred_alias")
        .add_attribute("address", address_trying_to_update)
        .add_attribute("username", token_id);
    Ok(res)
}

//
// --- we override these purely so we can clear any preferred aliases on transfer or burn
//

// fn clear_aliases(deps: DepsMut, token_id: String) -> Result<(), ContractError> {
//     let contract = Cw721MetadataContract::default();
//     let username_nft = contract.tokens.load(deps.storage, &token_id)?;
//     let res = PRIMARY_ALIASES.remove(deps.storage, &username_nft.owner);
//     Ok(res)
// }

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

pub fn transfer_nft(
    contract: Cw721MetadataContract,
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    token_id: String,
) -> Result<Response, ContractError> {
    // clear aliases before transfer iif it is the one being xfrd
    clear_alias_if_primary(deps.branch(), token_id.to_string())?;

    // blank meta before xfer
    clear_metadata(deps.branch(), token_id.to_string())?;

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
    // clear aliases before send iif it is the one being sent
    clear_alias_if_primary(deps.branch(), token_id.to_string())?;

    // blank meta before send
    clear_metadata(deps.branch(), token_id.to_string())?;

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

    contract.tokens.remove(deps.storage, &token_id)?;
    contract.decrement_tokens(deps.storage)?;

    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("sender", info.sender)
        .add_attribute("token_id", token_id))
}
