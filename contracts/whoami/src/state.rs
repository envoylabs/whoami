use crate::msg::{ContractInfo, MintingFeesResponse, Service};
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

// this is a mapping of address to token_id
pub const PRIMARY_ALIASES: Map<&Addr, String> = Map::new("aliases");

// this is the legacy contract info
// you should no longer write to it
pub const LEGACY_CONTRACT_INFO: Item<ContractInfo> = Item::new("contract_info");

// this is the contract info
pub const CONTRACT_INFO: Item<ContractInfo> = Item::new("whoami_contract_info");

// this is the username length cap
pub const USERNAME_LENGTH_CAP: Item<u32> = Item::new("username_length_cap");

// this is fees info
pub const MINTING_FEES_INFO: Item<MintingFeesResponse> = Item::new("minting_fees");

// the did method (namespace)
pub const DID_METHOD: Item<String> = Item::new("did_method");

// the default service endpoint
pub const DEFAULT_SERVICE: Item<Service> = Item::new("default_service");
