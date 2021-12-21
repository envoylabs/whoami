use cosmwasm_std::{Binary, Uint128};
use cw20::Logo;
use cw721::Expiration;
use cw721_base::{
    msg::ExecuteMsg as CW721ExecuteMsg, MintMsg as CW721MintMsg, QueryMsg as CW721QueryMsg,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SurchargeInfo {
    /// Max characters that are affected by the surcharge
    /// e.g. 5
    pub surcharge_max_characters: u32,
    /// The surcharge fee. This plus any base mint fee
    /// add up to the total fixed cost of minting an NFT username
    /// this is assumed to be in native_denom
    /// for now, no other option is available, so if you e.g.
    /// want 1 ATOM, use 1000000 as this value (i.e. it is uatom)
    pub surcharge_fee: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Name of the NFT contract
    pub name: String,
    /// Symbol of the NFT contract
    pub symbol: String,

    /// The token name of the native denom, e.g. ujuno uatom
    pub native_denom: String,
    /// The decimals of the token
    /// Same format as decimals above, eg. if it is uatom, where 1 unit is 10^-6 ATOM, use 6 here
    pub native_decimals: u8,

    /// Is there a token cap for this contract?
    /// i.e. a cap for number of tokens an address can manage
    /// it's a blunt tool against hoarding.
    pub token_cap: Option<u32>,

    /// An optional fee, paid to the admin_address
    /// half is burned by default, you have to override this
    /// in mint if that's not ok with you
    pub base_mint_fee: Option<Uint128>,

    /// An optional percentage of the mint fee to burn
    pub burn_percentage: Option<u64>,

    /// An optional surcharge for short names
    /// e.g. anything below 5 gets an additional charge
    /// this plus base_mint_fee are combined to come up
    /// with a total mint fee
    /// this is assumed to be in native_denom
    /// for now, no other option is available, so if you e.g.
    /// want 1 ATOM, use 1000000 as this value (i.e. it is uatom)
    pub short_name_surcharge: Option<SurchargeInfo>,

    /// The admin address for the contract
    /// replaces the minter field as minting is permissionless
    pub admin_address: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<Logo>,
    pub email: Option<String>,
    pub external_url: Option<String>,
    pub public_name: Option<String>,
    pub public_bio: Option<String>,
    pub twitter_id: Option<String>,
    pub discord_id: Option<String>,
    pub telegram_id: Option<String>,
    pub keybase_id: Option<String>,
    pub validator_operator_address: Option<String>,
    /// For future compatibility, we want to support
    /// a recursive lookup of tokens that constitutes a path
    /// somewhat like a DNS
    /// if this is None then it is a base token
    pub parent_token_id: Option<String>,
}

pub type Extension = Metadata;

pub type MintMsg = CW721MintMsg<Extension>;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct UpdateMetadataMsg {
    pub token_id: String,
    pub metadata: Metadata,
}

/// This can only be done by the contract admin
/// Note that these fields will forcibly update what is already set
/// You must be declarative and specify exactly the new desired behaviour
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct UpdateMintingFeesMsg {
    pub token_cap: Option<u32>,
    pub base_mint_fee: Option<Uint128>,
    pub burn_percentage: Option<u64>,
    pub short_name_surcharge: Option<SurchargeInfo>,
}

// Extended CW721 ExecuteMsg, added the ability to update, burn, and finalize nft
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Updates the minting fees configured at contract instantiation
    UpdateMintingFees(UpdateMintingFeesMsg),

    /// If the sender has multiple NFTs or aliases, they will want to set a default. This allows them to do this.
    UpdatePrimaryAlias { token_id: String },

    /// Updates the metadata of the NFT
    UpdateMetadata(UpdateMetadataMsg),

    /// Destroys the NFT permanently.
    Burn { token_id: String },

    /// Set admin
    SetAdminAddress { admin_address: String },

    /// Mint a new NFT
    Mint(MintMsg),

    // Standard CW721 ExecuteMsg
    /// Transfer is a base message to move a token to another account without triggering actions
    TransferNft { recipient: String, token_id: String },
    /// Send is a base message to transfer a token to a contract and trigger an action
    /// on the receiving contract.
    SendNft {
        contract: String,
        token_id: String,
        msg: Binary,
    },
    /// Allows operator to transfer / send the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve {
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted Approval
    Revoke { spender: String, token_id: String },
    /// Allows operator to transfer / send any token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    ApproveAll {
        operator: String,
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll { operator: String },
}

impl From<ExecuteMsg> for CW721ExecuteMsg<Extension> {
    fn from(msg: ExecuteMsg) -> CW721ExecuteMsg<Extension> {
        match msg {
            ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => CW721ExecuteMsg::TransferNft {
                recipient,
                token_id,
            },
            ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => CW721ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            },
            ExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            } => CW721ExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            },
            ExecuteMsg::Revoke { spender, token_id } => {
                CW721ExecuteMsg::Revoke { spender, token_id }
            }
            ExecuteMsg::ApproveAll { operator, expires } => {
                CW721ExecuteMsg::ApproveAll { operator, expires }
            }
            ExecuteMsg::RevokeAll { operator } => CW721ExecuteMsg::RevokeAll { operator },
            _ => panic!("cannot covert {:?} to CW721ExecuteMsg", msg),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Query preferred alias. Takes an address and returns the token id, if set.
    /// default behaviour is to return the first result, if unset.
    PrimaryAlias { address: String },
    /// Return the owner of the given token, error if token does not exist
    /// Return type: OwnerOfResponse
    OwnerOf {
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },
    /// List all operators that can access all of the owner's tokens.
    /// Return type: `OperatorsResponse`
    AllOperators {
        owner: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Total number of tokens issued
    NumTokens {},

    /// With MetaData Extension.
    /// Returns top-level metadata about the contract: `ContractInfoResponse`
    ContractInfo {},
    /// With MetaData Extension.
    /// Returns metadata about one particular token, based on *ERC721 Metadata JSON Schema*
    /// but directly from the contract: `NftInfoResponse`
    NftInfo { token_id: String },
    /// With MetaData Extension.
    /// Returns the result of both `NftInfo` and `OwnerOf` as one query as an optimization
    /// for clients: `AllNftInfo`
    AllNftInfo {
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },

    /// With Enumerable extension.
    /// Returns all tokens owned by the given address, [] if unset.
    /// Return type: TokensResponse.
    Tokens {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// With Enumerable extension.
    /// Requires pagination. Lists all token_ids controlled by the contract.
    /// Return type: TokensResponse.
    AllTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Return the admin address
    AdminAddress {},
}

impl From<QueryMsg> for CW721QueryMsg {
    fn from(msg: QueryMsg) -> CW721QueryMsg {
        match msg {
            QueryMsg::AdminAddress {} => CW721QueryMsg::Minter {},
            QueryMsg::OwnerOf {
                token_id,
                include_expired,
            } => CW721QueryMsg::OwnerOf {
                token_id,
                include_expired,
            },
            QueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            } => CW721QueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            },
            QueryMsg::NumTokens {} => CW721QueryMsg::NumTokens {},
            QueryMsg::NftInfo { token_id } => CW721QueryMsg::NftInfo { token_id },
            QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            } => CW721QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            },
            QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            } => CW721QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            },
            QueryMsg::AllTokens { start_after, limit } => {
                CW721QueryMsg::AllTokens { start_after, limit }
            }
            _ => panic!("cannot covert {:?} to CW721QueryMsg", msg),
        }
    }
}

// returns a token_id (i.e. a username)
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PrimaryAliasResponse {
    pub username: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    pub name: String,
    pub symbol: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfoResponse {
    pub name: String,
    pub symbol: String,
    pub native_denom: String,
    pub native_decimals: u8,
    pub token_cap: Option<u32>,
    pub base_mint_fee: Option<Uint128>,
    pub burn_percentage: Option<u64>,
    pub short_name_surcharge: Option<SurchargeInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MintingFeesResponse {
    pub native_denom: String,
    pub native_decimals: u8,
    pub token_cap: Option<u32>,
    pub base_mint_fee: Option<Uint128>,
    pub burn_percentage: Option<u64>,
    pub short_name_surcharge: Option<SurchargeInfo>,
}
