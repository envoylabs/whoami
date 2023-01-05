use cw_storage_plus::{Map, Item};
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use whoami_did::msg::{DidDocument};

pub const CONFIG: Item<Config> = Item::new("config");
pub const DID_DOCUMENTS: Map<String, DidDocument> = Map::new("dids");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    // did spec - https://www.w3.org/TR/did-core/#dfn-did-methods
    pub did_method: String
}