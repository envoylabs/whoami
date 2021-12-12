use crate::msg::ContractInfoResponse;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

// this is a mapping of address to token_id
pub const PREFERRED_ALIASES: Map<&Addr, String> = Map::new("aliases");

// this is the contract info
pub const CONTRACT_INFO: Item<ContractInfoResponse> = Item::new("contract_info");
