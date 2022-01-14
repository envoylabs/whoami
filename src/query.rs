use crate::msg::{
    ContractInfoResponse, GetParentIdResponse, GetPathResponse, IsContractResponse,
    PrimaryAliasResponse, WhoamiNftInfoResponse,
};
use crate::state::{CONTRACT_INFO, MINTING_FEES_INFO, PRIMARY_ALIASES};
use crate::Cw721MetadataContract;
use cosmwasm_std::{Deps, Env, Order, StdError, StdResult};
use cw721::TokensResponse;
use cw_storage_plus::Bound;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

fn get_tokens_for_owner(
    contract: Cw721MetadataContract,
    deps: Deps,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let owner_addr = deps.api.addr_validate(&owner)?;
    let tokens: Vec<String> = contract
        .tokens
        .idx
        .owner
        .prefix(owner_addr)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|x| x.map(|addr| addr.to_string()))
        .collect::<StdResult<Vec<_>>>()?;

    Ok(TokensResponse { tokens })
}

fn get_first_token_for_owner(
    contract: Cw721MetadataContract,
    deps: Deps,
    owner: String,
) -> StdResult<String> {
    let tokens_response = get_tokens_for_owner(contract, deps, owner, None, Some(1))?;
    let first_token = tokens_response.tokens[0].clone();
    Ok(first_token)
}

// note we call this PRIMARY in the UI
pub fn primary_alias(
    contract: Cw721MetadataContract,
    deps: Deps,
    _env: Env,
    owner: String,
) -> StdResult<PrimaryAliasResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let existing_alias = PRIMARY_ALIASES.may_load(deps.storage, &owner_addr)?;

    // if nothing returned, get first
    let username = match existing_alias {
        Some(alias) => alias,
        None => get_first_token_for_owner(contract, deps, owner)?,
    };
    Ok(PrimaryAliasResponse { username })
}

pub fn contract_info(deps: Deps) -> StdResult<ContractInfoResponse> {
    let contract_info = CONTRACT_INFO.load(deps.storage)?;
    let minting_fees = MINTING_FEES_INFO.load(deps.storage)?;

    let contract_info_response = ContractInfoResponse {
        name: contract_info.name,
        symbol: contract_info.symbol,
        native_denom: minting_fees.native_denom,
        native_decimals: minting_fees.native_decimals,
        token_cap: minting_fees.token_cap,
        base_mint_fee: minting_fees.base_mint_fee,
        burn_percentage: minting_fees.burn_percentage,
        short_name_surcharge: minting_fees.short_name_surcharge,
    };
    Ok(contract_info_response)
}

pub fn is_contract(
    contract: Cw721MetadataContract,
    deps: Deps,
    token_id: String,
) -> StdResult<IsContractResponse> {
    let token = contract.tokens.load(deps.storage, &token_id)?;
    if let Some(addr) = token.extension.contract_address {
        Ok(IsContractResponse {
            contract_address: addr,
        })
    } else {
        Err(StdError::NotFound {
            kind: "No contract address".to_string(),
        })
    }
}

// looks up the actual token
// so throws an error if it doesn't exist
pub fn get_parent_id(
    contract: Cw721MetadataContract,
    deps: Deps,
    token_id: String,
) -> StdResult<GetParentIdResponse> {
    let token = contract.tokens.load(deps.storage, &token_id)?;

    match token.extension.parent_token_id {
        Some(pti) => {
            // attempt to load parent
            // else error
            let _parent_token = contract.tokens.load(deps.storage, &pti)?;

            Ok(GetParentIdResponse {
                parent_token_id: pti,
            })
        }
        None => Err(StdError::NotFound {
            kind: "Parent not found".to_string(),
        }),
    }
}

pub fn get_parent_nft_info(
    contract: Cw721MetadataContract,
    deps: Deps,
    token_id: String,
) -> StdResult<WhoamiNftInfoResponse> {
    let token = contract.tokens.load(deps.storage, &token_id)?;

    match token.extension.parent_token_id {
        Some(pti) => {
            // attempt to load parent
            let parent_token = contract.tokens.load(deps.storage, &pti)?;

            Ok(WhoamiNftInfoResponse {
                token_uri: parent_token.token_uri,
                extension: parent_token.extension,
            })
        }
        None => Err(StdError::NotFound {
            kind: "Parent not found".to_string(),
        }),
    }
}

// get full path by heading up through the parents
pub fn get_path(
    contract: Cw721MetadataContract,
    deps: Deps,
    token_id: String,
) -> StdResult<GetPathResponse> {
    let token = contract.tokens.load(deps.storage, &token_id)?;

    let mut parents = vec![token_id];
    let mut current_parent_token_id = token.extension.parent_token_id;

    while current_parent_token_id.is_some() {
        let cpti = current_parent_token_id.unwrap();

        // look up parent token
        let parent_token = contract.tokens.load(deps.storage, &cpti)?;

        // insert current token
        parents.insert(0, cpti);

        // set the next one - this will be Some or None
        current_parent_token_id = parent_token.extension.parent_token_id;
    }

    let path = parents.join("/");
    Ok(GetPathResponse { path })
}
