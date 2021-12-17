use crate::msg::MintingFeesResponse;
use cosmwasm_std::{
    coins, Addr, BankMsg, CosmosMsg, Decimal, DepsMut, Order, Response, StdError, StdResult,
    Uint128,
};
use cw20::{EmbeddedLogo, Logo};
use cw721_base::ContractError;

use crate::Cw721MetadataContract;

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
    let pks: Vec<_> = contract
        .tokens
        .idx
        .owner
        .prefix(address)
        .keys(deps.storage, None, None, Order::Ascending)
        .take(default_limit) // set default big limit
        .collect();

    let res: Result<Vec<_>, _> = pks.iter().map(|v| String::from_utf8(v.to_vec())).collect();
    let owned_tokens = res.map_err(StdError::invalid_utf8)?;
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
        Logo::Url(_) => Err(ContractError::Unauthorized {}), // this is an embedded field, we don't allow URLs like CW20
    }
}
