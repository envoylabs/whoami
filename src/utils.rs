use crate::error::ContractError;
use crate::msg::MintingFeesResponse;
use crate::state::USERNAME_LENGTH_CAP;
use cosmwasm_std::{
    coins, Addr, BankMsg, CosmosMsg, Decimal, Deps, DepsMut, Order, Response, StdError, StdResult,
    Uint128,
};
use cw20::{EmbeddedLogo, Logo};

use crate::Cw721MetadataContract;
use regex::Regex;
use std::convert::TryFrom;

// dumb, granted
// but a basic sense check
// that clients don't submit something random
// obviously if the key is the correct format
// but still in some way incorrect it will error later
pub fn pgp_pubkey_format_is_valid(pgp_pubkey: &str) -> bool {
    let first_check_passed = str::starts_with(pgp_pubkey, "-----BEGIN PGP PUBLIC KEY BLOCK-----");
    let second_check_passed = str::ends_with(pgp_pubkey, "-----END PGP PUBLIC KEY BLOCK-----");

    first_check_passed && second_check_passed
}

// for a subdomain, we need to validate:
// first, is the parent_token_id an actual token?
// if it's not, throw an error
// second, is the parent_token_id owned by the same owner?
// if not, throw an error
pub fn validate_subdomain(
    contract: &Cw721MetadataContract,
    deps: &DepsMut,
    parent_token_id: String,
    minter: Addr,
) -> Result<(), ContractError> {
    // check one - load
    let parent_token = contract.tokens.load(deps.storage, &parent_token_id)?;

    // check two
    if minter != parent_token.owner {
        return Err(ContractError::Unauthorized {});
    }

    Ok(())
}

pub fn get_username_length(username: &str) -> u32 {
    u32::try_from(username.chars().count()).unwrap()
}

// validate username length. this, or to some number of bytes?
pub fn validate_username_length(deps: Deps, username: &str) -> bool {
    let username_length_cap = USERNAME_LENGTH_CAP.may_load(deps.storage).unwrap();

    let cap = username_length_cap.unwrap_or(20);
    let username_length = get_username_length(username);

    username_length <= cap
}

pub fn validate_username_characters(username: &str) -> bool {
    // first check for any characters _other than_ allowed characters
    let invalid_characters: Regex = Regex::new(r"[^a-z0-9_\-]").unwrap();
    let first_check_passed = !invalid_characters.is_match(username);

    // then check for invalid sequence of hyphens or underscores
    // if is_match returns true, it is invalid
    let invalid_hyphens_underscores: Regex = Regex::new(r"[_\-]{2,}").unwrap();
    let second_check_passed = !invalid_hyphens_underscores.is_match(username);

    first_check_passed && second_check_passed
}

pub fn username_is_valid(deps: Deps, username: &str) -> bool {
    let username_length_valid = validate_username_length(deps, username);
    let username_characters_valid = validate_username_characters(username);
    username_characters_valid && username_length_valid
}

// initially we allowed paths like employment/death-star-1
// but it makes more sense to use the fact we use :: to namespace paths
// to always resolve the parent
pub fn validate_path_characters(path: &str, parent_token_id: &str) -> bool {
    // first check for any characters _other than_ allowed characters
    let invalid_characters: Regex = Regex::new(r"[^a-z0-9_\-]").unwrap();
    let first_check_passed = !invalid_characters.is_match(path);

    // then check for invalid sequence of hyphens or underscores
    // if is_match returns true, it is invalid
    let invalid_hyphens_underscores: Regex = Regex::new(r"[_\-/]{2,}").unwrap();
    let second_check_passed = !invalid_hyphens_underscores.is_match(path);

    // check parent token isn't in there
    let parent_token_id_present: Regex = Regex::new(parent_token_id).unwrap();
    let third_check_passed = !parent_token_id_present.is_match(path);

    let leading_special_chars: Regex = Regex::new(r"^[_\-]").unwrap();
    let fourth_check_passed = !leading_special_chars.is_match(path);

    let trailing_special_chars: Regex = Regex::new(r"[_\-]$").unwrap();
    let fifth_check_passed = !trailing_special_chars.is_match(path);

    first_check_passed
        && second_check_passed
        && third_check_passed
        && fourth_check_passed
        && fifth_check_passed
}

pub fn path_is_valid(path: &str, parent_token_id: &str) -> bool {
    let path_length = u32::try_from(path.chars().count()).unwrap();
    let path_length_valid = path_length <= 2048;
    let path_characters_valid = validate_path_characters(path, parent_token_id);
    path_characters_valid && path_length_valid
}

pub fn is_path(token_id: &str) -> bool {
    token_id.contains("::")
}

// check whether the offered token id matches the namespace
// this specifically checks if it is at the beginning of the string
// however note we actually do not allow the parent id anywhere in
// the substring thanks to the validator fn - this prevents cycles
pub fn namespace_in_path(token_id: &str, parent_token_id: &str) -> bool {
    // okay so this is meant to identify a namespace
    // as part of a longer path
    // as such there must be _at least_ one more character
    // after the namespace. moreover, it _should be a separator_, '::'
    let namespace_regex = format!("^{}::", parent_token_id);
    let has_namespace: Regex = Regex::new(&namespace_regex).unwrap();
    has_namespace.is_match(token_id)
}

// if it is a path, removes the namespace
// otherwise leaves it untouched
pub fn remove_namespace_from_path(path: &str, parent_token_id: &str) -> String {
    let parent_id_regex = Regex::new(parent_token_id).unwrap();
    parent_id_regex.replace_all(path, "").to_string()
}

pub fn get_mint_fee(minting_fees: MintingFeesResponse, username_length: u32) -> Option<Uint128> {
    // is token name short enough to trigger a surcharge?
    let surcharge_is_owed = match minting_fees.short_name_surcharge {
        Some(ref sc) => username_length < sc.surcharge_max_characters,
        None => false,
    };

    match minting_fees.base_mint_fee {
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
    }
}

pub fn get_number_of_owned_tokens(
    contract: &Cw721MetadataContract,
    deps: &DepsMut,
    address: Addr,
    default_limit: usize,
) -> StdResult<usize> {
    let owned_tokens: Vec<String> = contract
        .tokens
        .idx
        .owner
        .prefix(address)
        .keys(deps.storage, None, None, Order::Ascending)
        .take(default_limit) // set default big limit
        .map(|x| x.map(|addr| addr.to_string()))
        .collect::<StdResult<Vec<_>>>()?;

    let number_of_tokens_owned = owned_tokens.len();
    Ok(number_of_tokens_owned)
}

pub fn get_mint_response(
    admin_address: Addr,
    mint_message_sender: Addr,
    native_denom: String,
    fee: Option<Uint128>,
    burn_percentage: Option<u64>,
    token_id: String,
) -> Response {
    match fee {
        Some(fee) => {
            let msgs: Vec<CosmosMsg> = match burn_percentage {
                Some(bp) => {
                    let fee_to_admin = fee * Decimal::percent(100 - bp);
                    let fee_to_burn = fee * Decimal::percent(bp);
                    vec![
                        BankMsg::Send {
                            to_address: admin_address.to_string(),
                            amount: coins(fee_to_admin.u128(), native_denom.clone()),
                        }
                        .into(),
                        BankMsg::Burn {
                            amount: coins(fee_to_burn.u128(), native_denom),
                        }
                        .into(),
                    ]
                }
                None => vec![BankMsg::Send {
                    to_address: admin_address.to_string(),
                    amount: coins(fee.u128(), native_denom),
                }
                .into()],
            };

            Response::new()
                .add_attribute("action", "mint")
                .add_attribute("minter", mint_message_sender)
                .add_attribute("token_id", token_id)
                .add_messages(msgs)
        }
        None => Response::new()
            .add_attribute("action", "mint")
            .add_attribute("minter", mint_message_sender)
            .add_attribute("token_id", token_id),
    }
}

// -- logo helpers as they're not public in CW20 --
const LOGO_SIZE_CAP: usize = 10 * 1024;

/// Checks if data starts with XML preamble
fn verify_xml_preamble(data: &[u8]) -> Result<(), ContractError> {
    // The easiest way to perform this check would be just match on regex, however regex
    // compilation is heavy and probably not worth it.

    let preamble = data.split_inclusive(|c| *c == b'>').next().ok_or_else(|| {
        ContractError::Std(StdError::ParseErr {
            msg: "Failed to parse SVG".to_string(),
            target_type: "Logo".to_string(),
        })
    })?;

    const PREFIX: &[u8] = b"<?xml ";
    const POSTFIX: &[u8] = b"?>";

    if !(preamble.starts_with(PREFIX) && preamble.ends_with(POSTFIX)) {
        Err(ContractError::Std(StdError::ParseErr {
            msg: "Failed to parse SVG".to_string(),
            target_type: "Logo".to_string(),
        }))
    } else {
        Ok(())
    }

    // Additionally attributes format could be validated as they are well defined, as well as
    // comments presence inside of preable, but it is probably not worth it.
}

/// Validates XML logo
fn verify_xml_logo(logo: &[u8]) -> Result<(), ContractError> {
    verify_xml_preamble(logo)?;

    if logo.len() > LOGO_SIZE_CAP {
        Err(ContractError::Std(StdError::ParseErr {
            msg: "Failed to parse SVG - too large".to_string(),
            target_type: "Logo".to_string(),
        }))
    } else {
        Ok(())
    }
}

/// Validates png logo
fn verify_png_logo(logo: &[u8]) -> Result<(), ContractError> {
    // PNG header format:
    // 0x89 - magic byte, out of ASCII table to fail on 7-bit systems
    // "PNG" ascii representation
    // [0x0d, 0x0a] - dos style line ending
    // 0x1a - dos control character, stop displaying rest of the file
    // 0x0a - unix style line ending
    const HEADER: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if logo.len() > LOGO_SIZE_CAP {
        Err(ContractError::Std(StdError::ParseErr {
            msg: "Failed to parse PNG - too large".to_string(),
            target_type: "Logo".to_string(),
        }))
    } else if !logo.starts_with(&HEADER) {
        Err(ContractError::Std(StdError::ParseErr {
            msg: "Failed to parse PNG".to_string(),
            target_type: "Logo".to_string(),
        }))
    } else {
        Ok(())
    }
}

/// Checks if passed logo is correct, and if not, returns an error
pub fn verify_logo(logo: &Logo) -> Result<(), ContractError> {
    match logo {
        Logo::Embedded(EmbeddedLogo::Svg(logo)) => verify_xml_logo(logo),
        Logo::Embedded(EmbeddedLogo::Png(logo)) => verify_png_logo(logo),
        Logo::Url(_) => Err(ContractError::NoLinksPermitted {}), // this is an embedded field, we don't allow URLs like CW20
    }
}
