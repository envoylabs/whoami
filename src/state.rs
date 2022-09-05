use crate::msg::{ContractInfo, MintingFeesResponse};
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

// this is a mapping of address to token_id
pub const PRIMARY_ALIASES: Map<&Addr, String> = Map::new("aliases");

// this is the contract info
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("whoami_contract_info");

// this is the username length cap
pub const USERNAME_LENGTH_CAP: Item<u32> = Item::new("username_length_cap");

// this is fees info
pub const MINTING_FEES_INFO: Item<MintingFeesResponse> = Item::new("minting_fees");
