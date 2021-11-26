use crate::msg::MintMsg;
use crate::state::PREFERRED_ALIASES;
use crate::Cw721MetadataContract;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw721_base::state::TokenInfo;
use cw721_base::ContractError;

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

    // validate owner addr
    let owner_address = deps.api.addr_validate(&msg.owner)?;

    // username == token_id
    // validate username length. this, or to some number of bytes?
    let username = &msg.token_id;
    if username.chars().count() > 20 {
        return Err(ContractError::Unauthorized {});
    }

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

    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_attribute("minter", info.sender)
        .add_attribute("token_id", msg.token_id))
}

// takes a mintmsg and uses the data therein to update the corresponding NFT metadata, if allowed.
// pub fn update_metadata(
//     contract: Cw721MetadataContract,
//     deps: DepsMut,
//     _env: Env,
//     info: MessageInfo,
//     msg: MintMsg<Extension>,
// ) -> Result<Response, ContractError> {
// }

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
    let address_trying_to_update = info.sender.clone();
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

// --- we override these purely so we can clear any preferred aliases on transfer or burn
