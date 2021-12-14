use cosmwasm_std::{
    coins, BankMsg, Binary, CosmosMsg, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult,
};
use cw2::set_contract_version;
use cw721::Cw721ReceiveMsg;
use cw721_base::state::TokenInfo;
use cw721_base::ContractError;
use std::convert::TryFrom;
use std::convert::TryInto;

use crate::msg::{
    ContractInfo, InstantiateMsg, MintMsg, MintingFeesResponse, UpdateMetadataMsg,
    UpdateMintingFeesMsg,
};
use crate::state::{CONTRACT_INFO, MINTING_FEES_INFO, PREFERRED_ALIASES};
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

    // get minting fees and minter (i.e. admin)
    let minting_fees = MINTING_FEES_INFO.load(deps.storage)?;
    let admin_address = contract.minter(deps.as_ref())?.minter;

    // check if trying to mint too many
    // who can need more than 20?
    let default_limit = 20;
    let pks: Vec<_> = contract
        .tokens
        .idx
        .owner
        .prefix(address_trying_to_mint)
        .keys(deps.storage, None, None, Order::Ascending)
        .take(default_limit) // set default big limit
        .collect();

    let res: Result<Vec<_>, _> = pks.iter().map(|v| String::from_utf8(v.to_vec())).collect();
    let owned_tokens = res.map_err(StdError::invalid_utf8)?;
    let number_of_tokens_owned = owned_tokens.len();

    // error out if we exceed configured cap or we already
    // have the default max
    match minting_fees.token_cap {
        Some(tc) => {
            if number_of_tokens_owned > tc.try_into().unwrap() {
                return Err(ContractError::Unauthorized {});
            }
        }
        None => {
            if number_of_tokens_owned == default_limit {
                return Err(ContractError::Unauthorized {});
            }
        }
    }

    // validate owner addr
    let owner_address = deps.api.addr_validate(&msg.owner)?;

    // username == token_id
    // validate username length. this, or to some number of bytes?
    let username = &msg.token_id;
    let username_length = u32::try_from(username.chars().count()).unwrap();
    if username_length > 20 {
        return Err(ContractError::Unauthorized {});
    }

    // is token name short enough to trigger a surcharge?
    let surcharge_is_owed = match minting_fees.short_name_surcharge {
        Some(ref sc) => username_length < sc.surcharge_max_characters,
        None => false,
    };

    // work out what fees are owed
    let fee = match minting_fees.base_mint_fee {
        Some(base_fee) => match minting_fees.short_name_surcharge {
            Some(sc) => {
                if surcharge_is_owed {
                    let summed = base_fee + sc.surcharge_fee; // if both, sum
                    Some(summed)
                } else {
                    Some(base_fee) // username is long, no sc owed
                }
            }
            None => Some(base_fee), // just fee, no sc is configured
        },
        None => match minting_fees.short_name_surcharge {
            // no base fee
            Some(sc) => {
                if surcharge_is_owed {
                    Some(sc.surcharge_fee) // just surcharge
                } else {
                    None // neither owed
                }
            }
            None => None, // neither owed
        },
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
    let res = match fee {
        Some(fee) => {
            let msgs: Vec<CosmosMsg> = vec![BankMsg::Send {
                to_address: admin_address,
                amount: coins(fee.u128(), minting_fees.native_denom),
            }
            .into()];

            Response::new()
                .add_attribute("action", "mint")
                .add_attribute("minter", info.sender)
                .add_attribute("token_id", msg.token_id)
                .add_messages(msgs)
        }
        None => Response::new()
            .add_attribute("action", "mint")
            .add_attribute("minter", info.sender)
            .add_attribute("token_id", msg.token_id),
    };
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
pub fn update_preferred_alias(
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
    PREFERRED_ALIASES.save(deps.storage, &address_trying_to_update, &token_id)?;

    let res = Response::new()
        .add_attribute("action", "update_preferred_alias")
        .add_attribute("address", address_trying_to_update)
        .add_attribute("username", token_id);
    Ok(res)
}

//
// --- we override these purely so we can clear any preferred aliases on transfer or burn
//

// fn clear_aliases(
//     contract: Cw721MetadataContract,
//     deps: DepsMut,
//     token_id: String,
// ) -> Result<(), ContractError> {
//     let username_nft = contract.tokens.load(deps.storage, &token_id)?;
//     let res = PREFERRED_ALIASES.remove(deps.storage, &username_nft.owner);
//     Ok(res)
// }

pub fn transfer_nft(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    token_id: String,
) -> Result<Response, ContractError> {
    // clear aliases before transfer
    let username_nft = contract.tokens.load(deps.storage, &token_id)?;
    PREFERRED_ALIASES.remove(deps.storage, &username_nft.owner);

    contract._transfer_nft(deps, &env, &info, &recipient, &token_id)?;

    Ok(Response::new()
        .add_attribute("action", "transfer_nft")
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", recipient)
        .add_attribute("token_id", token_id))
}

pub fn send_nft(
    contract: Cw721MetadataContract,
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    receiving_contract: String,
    token_id: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    // clear aliases before send
    let username_nft = contract.tokens.load(deps.storage, &token_id)?;
    PREFERRED_ALIASES.remove(deps.storage, &username_nft.owner);

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
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let token = contract.tokens.load(deps.storage, &token_id)?;
    contract.check_can_send(deps.as_ref(), &env, &info, &token)?;

    // clear aliases before delete
    PREFERRED_ALIASES.remove(deps.storage, &token.owner);

    contract.tokens.remove(deps.storage, &token_id)?;
    contract.decrement_tokens(deps.storage)?;

    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("sender", info.sender)
        .add_attribute("token_id", token_id))
}
