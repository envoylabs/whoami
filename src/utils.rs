use cosmwasm_std::{
    coins, Addr, BankMsg, CosmosMsg, Decimal, DepsMut, Order, Response, StdError, StdResult,
    Uint128,
};

use crate::msg::MintingFeesResponse;

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
    token_id: String,
) -> Response {
    match fee {
        Some(fee) => {
            let half_of_fee = fee * Decimal::percent(50);
            let msgs: Vec<CosmosMsg> = vec![
                BankMsg::Send {
                    to_address: admin_address.to_string(),
                    amount: coins(half_of_fee.u128(), native_denom.clone()),
                }
                .into(),
                BankMsg::Burn {
                    amount: coins(half_of_fee.u128(), native_denom),
                }
                .into(),
            ];

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
