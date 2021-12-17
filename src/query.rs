use crate::msg::{ContractInfoResponse, PreferredAliasResponse};
use crate::state::{CONTRACT_INFO, MINTING_FEES_INFO, PREFERRED_ALIASES};
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
    let pks: Vec<_> = contract
        .tokens
        .idx
        .owner
        .prefix(owner_addr)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect();

    let res: Result<Vec<_>, _> = pks.iter().map(|v| String::from_utf8(v.to_vec())).collect();
    let tokens = res.map_err(StdError::invalid_utf8)?;
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
pub fn preferred_alias(
    contract: Cw721MetadataContract,
    deps: Deps,
    _env: Env,
    owner: String,
) -> StdResult<PreferredAliasResponse> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let existing_alias = PREFERRED_ALIASES.may_load(deps.storage, &owner_addr)?;

    // if nothing returned, get first
    let username = match existing_alias {
        Some(alias) => alias,
        None => get_first_token_for_owner(contract, deps, owner)?,
    };
    Ok(PreferredAliasResponse { username })
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
