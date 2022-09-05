use crate::msg::{
    AddressOfResponse, ContractInfoResponse, GetParentIdResponse, GetPathResponse,
    IsContractResponse, ListUserInfoResponse, PrimaryAliasResponse, UserInfo,
    WhoamiNftInfoResponse,
};
use crate::state::{CONTRACT_INFO, MINTING_FEES_INFO, PRIMARY_ALIASES};
use crate::utils::{is_path, namespace_in_path, remove_namespace_from_path};
use crate::Cw721MetadataContract;
use cosmwasm_std::{Deps, Env, Order, StdError, StdResult};
use cw721::TokensResponse;
use cw_storage_plus::Bound;

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

fn get_tokens(
    contract: Cw721MetadataContract,
    deps: Deps,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<String>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

    let owner_addr = deps.api.addr_validate(&owner)?;
    let tokens: Vec<String> = contract
        .tokens
        .idx
        .owner
        .prefix(owner_addr)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    Ok(tokens)
}

fn get_tokens_for_owner(
    contract: Cw721MetadataContract,
    deps: Deps,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let tokens = get_tokens(contract, deps, owner, start_after, limit)?;

    Ok(TokensResponse { tokens })
}

pub fn list_info_by_alias(
    contract: Cw721MetadataContract,
    deps: Deps,
    aliases: Vec<String>,
) -> StdResult<ListUserInfoResponse> {
    let users: Vec<UserInfo> = aliases
        .into_iter()
        .map(|alias| -> StdResult<UserInfo> {
            let info = contract.tokens.load(deps.storage, &alias)?;
            Ok(UserInfo {
                alias,
                owner: info.owner.to_string(),
                metadata: info.extension,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(ListUserInfoResponse { users })
}

pub fn get_base_tokens_for_owner(
    contract: Cw721MetadataContract,
    deps: Deps,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let tokens = get_tokens_for_owner(contract, deps, owner, start_after, limit)?.tokens;

    let non_path_tokens = tokens.into_iter().filter(|path| !is_path(path)).collect();

    Ok(TokensResponse {
        tokens: non_path_tokens,
    })
}

// get the first non_path token
fn get_first_token_for_owner(
    contract: Cw721MetadataContract,
    deps: Deps,
    owner: String,
) -> StdResult<String> {
    let tokens_response = get_base_tokens_for_owner(contract, deps, owner, None, Some(1))?;

    if tokens_response.tokens.is_empty() {
        Err(StdError::NotFound {
            kind: "Primary alias not found".to_string(),
        })
    } else {
        let first_token = &tokens_response.tokens[0];
        Ok(first_token.to_string())
    }
}

pub fn get_paths_for_owner(
    contract: Cw721MetadataContract,
    deps: Deps,
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let tokens = get_tokens_for_owner(contract, deps, owner, start_after, limit)?.tokens;

    let paths = tokens.into_iter().filter(|path| is_path(path)).collect();

    Ok(TokensResponse { tokens: paths })
}

// get only those namespaced under token_id
pub fn get_paths_for_owner_and_token(
    deps: Deps,
    owner: String,
    token_id: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let contract = Cw721MetadataContract::default();
    let tokens = get_tokens(contract, deps, owner, start_after, limit)?;

    let paths = tokens
        .into_iter()
        .filter(|path| is_path(path) && namespace_in_path(path, &token_id))
        .collect();

    Ok(TokensResponse { tokens: paths })
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
    println!("test");
    println!("{}", contract_info.name);
    println!("{}", contract_info.symbol);

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

// like owner_of
// but returns owner
// and contract address (or none)
pub fn address_of(
    contract: Cw721MetadataContract,
    deps: Deps,
    token_id: String,
) -> StdResult<AddressOfResponse> {
    let token = contract.tokens.load(deps.storage, &token_id)?;
    Ok(AddressOfResponse {
        owner: token.owner.to_string(),
        contract_address: token.extension.contract_address,
        validator_address: token.extension.validator_operator_address,
    })
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
// paths for top level names would be jeffvader/anothertoken::some-nested-path
// paths for nested paths would be like anothertoken::some-nested-path::second-nest
// now in the case of that second-nest path, its parent is anothertoken::some-nested-path
// essentially once you get into :: land you are into super fun recursive times
pub fn get_path(
    contract: Cw721MetadataContract,
    deps: Deps,
    token_id: String,
) -> StdResult<GetPathResponse> {
    let token = contract.tokens.load(deps.storage, &token_id)?;

    // clip front off token_id if it is a path
    let sanitized_token_id = match token.extension.parent_token_id {
        Some(ref pti) => remove_namespace_from_path(&token_id, pti),
        None => token_id,
    };
    let mut parents = vec![sanitized_token_id];
    let mut current_parent_token_id = token.extension.parent_token_id;

    while current_parent_token_id.is_some() {
        let cpti = current_parent_token_id.unwrap();

        // look up parent token
        let parent_token = contract.tokens.load(deps.storage, &cpti)?;

        // clip off the front if this is a path
        // i.e. jeffvader::employment will resolve
        // so we clip off jeffvader
        // not even sure this case _can_ happen, but still
        let sanitized_parent_token_id = match parent_token.extension.parent_token_id {
            Some(ref cppti) => remove_namespace_from_path(&cpti, cppti),
            None => cpti,
        };

        // insert current token
        parents.insert(0, sanitized_parent_token_id);

        // set the next one - this will be Some or None
        current_parent_token_id = parent_token.extension.parent_token_id;
    }

    // finally, ensure we have no instances of /:: after join
    let joined = parents.join("/");
    let path = joined.replace("/::", "::");
    Ok(GetPathResponse { path })
}
