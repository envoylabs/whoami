use cosmwasm_std::Addr;
use cw_storage_plus::Map;

// this is a mapping of address to token_id
pub const PREFERRED_ALIASES: Map<&Addr, String> = Map::new("aliases");
