use cosmwasm_std::{Binary, Uint128};
use cw20::Logo;
use cw721::{Expiration, NftInfoResponse};
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
pub struct MigrateMsg {
    /// Specify the version that we are migrating up to
    pub target_version: String,
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

    /// The cap for a username length
    /// can be updated later by the admin_address
    pub username_length_cap: Option<u32>,

    /// The DID namespace
    /// this should be set to 'dens' but hey, you do you
    pub did_method: String,
}

/// This is the service that this contract will be exposed via
/// it should be set on creation of the contract
/// question: should it be updateable?
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Service {
    /// Service type
    pub r#type: String,
    /// The endpoint to access this service
    pub service_endpoint: String,
}

/// We use the co-located pubkey by default
/// See base def in https://www.w3.org/TR/did-spec-registries/#property-names
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Ed25519Key {
    /// The key's id path, e.g. "did:example:123#key-1"
    pub id: String,
    /// The type of key "Ed25519VerificationKey2018"
    pub r#type: String,
    /// The controlling did "did:example:123"
    pub controller: String,
    /// Base 58 pubkey, e.g. "H3C2AVvLMv6gmMNam3uVAjZpfkcJCwDwnZn6z3wXmqPV"
    pub public_key_base_58: String,
}

/// We redefine PGP key to use the verification syntax
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct PgpKey {
    /// The key's id path, e.g. "did:example:123#key-1"
    pub id: String,
    /// The type of key "PgpVerificationKey2021"
    pub r#type: String,
    /// The controlling did "did:example:123"
    pub controller: String,
    /// The PGP key, e.g. "-----BEGIN PGP PUBLIC KEY BLOCK-----\r\nVersion: OpenPGP.js v4.9.0\r\nComment: https://openpgpjs.org\r\n\r\nxjMEXkm5LRYJKwYBBAHaRw8BAQdASmfrjYr7vrjwHNiBsdcImK397Vc3t4BL\r\nE8rnN......v6\r\nDw==\r\n=wSoi\r\n-----END PGP PUBLIC KEY BLOCK-----\r\n"
    pub public_key_pgp: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct BlockchainAccountId {
    /// The key's id path, e.g. "did:example:123#vm-3"
    pub id: String,
    /// The type of key e.g. "EcdsaSecp256k1RecoveryMethod2020"
    pub r#type: String,
    /// The controlling did "did:example:123"
    pub controller: String,
    /// The blockchain account id
    /// note this needs to be replaced if the key is
    /// transferred in any way
    pub blockchain_account_id: String,
}

// frey comment: not sure these belong as verification methods
// feels like they are something else, unless oracle is used to sign?
// /// The assumption here is that there will be some kind of token
// /// that can be stored that is analogous to a public key
// /// and is not sensitive if stored
// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
// pub struct ExternalAccountIdWithToken {
//     /// The key's id path, e.g. "did:example:123#vm-3"
//     pub id: String,
//     /// The service e.g. "keybase"
//     pub r#type: String,
//     /// The controlling did "did:example:123"
//     pub controller: String,
//     /// The account id on the external service
//     pub external_account_id: String,
//     /// Metadata: url of external service
//     pub external_account_url: String,
//     /// Token that it is safe to store
//     pub external_account_safe_public_token: String,
// }

// /// This relies much more heavily on auditing at the point of save
// /// it should be considered unsafe if anybody other than the signing DID
// /// can update this field. In fact, it should probably be considered unsafe in
// /// that case as well
// /// This is essentially just a self-declared piece of metadata
// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
// pub struct ExternalAccountId {
//     /// The key's id path, e.g. "did:example:123#vm-3"
//     pub id: String,
//     /// The service e.g. "keybase"
//     pub r#type: String,
//     /// The controlling did "did:example:123"
//     pub controller: String,
//     /// The account id on the external service
//     pub external_account_id: String,
//     /// Metadata: url of external service
//     pub external_account_url: String,
// }

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum VerificationMethod {
    Ed25519Key,
    PgpKey,
    BlockchainAcccountId,
}

impl Default for VerificationMethod {
    fn default() -> Self {
        VerificationMethod::BlockchainAcccountId
    }
}

/// The DID Document
/// Note that for our case we always insist that the controller
/// is the same thing as the subject
/// another way of approaching this would be to make the controller
/// the smart contract itself, or even a DAO
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct DidDocument {
    /// The LD context. Will always be "https://www.w3.org/ns/did/v1"
    pub context: String,
    /// This will be the did id
    /// it is auto-generated
    pub id: String,
    /// Verification Method
    /// at the very least, BlockchainAccountId is required
    /// that is why this is not an Option type
    pub verification_method: Vec<VerificationMethod>,
    /// Services. Not optional as at least one should be set
    /// when the contract is instantiated
    pub service: Vec<Service>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct Metadata {
    /// Original meta fields
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
    pub contract_address: Option<String>,

    /// For future compatibility, we want to support
    /// a recursive lookup of tokens that constitutes a path
    /// somewhat like a DNS
    /// if this is None then it is a base token
    pub parent_token_id: Option<String>,

    /// the DID document
    pub did_document: DidDocument,
}

pub type Extension = Metadata;

pub type MintMsg = CW721MintMsg<Extension>;

pub type WhoamiNftInfoResponse = NftInfoResponse<Extension>;

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

    /// Updates the username length cap
    UpdateUsernameLengthCap { new_length: u32 },

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

    /// Mint a new path NFT
    /// e.g a user has jeffvader
    /// they could mint the paths
    /// construction-projects::death-star-1
    /// construction-projects::current
    /// construction-projects::current::death-star-2
    /// and all could be resolved by GetFullPath to
    /// jeffvader::construction-projects::...
    MintPath(MintMsg),

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
    /// Query address of a name
    /// This returns contract address if contract
    AddressOf { token_id: String },
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
    /// Returns all namespace/base tokens owned by the given address,
    /// [] if unset.
    /// Return type: TokensResponse.
    BaseTokens {
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

    /// Return if this is an executable contract or not
    IsContract { token_id: String },

    /// Return the id for a parent for this token_id
    GetParentId { token_id: String },

    /// Return the NFT info for a parent of this token_id
    GetParentInfo { token_id: String },

    /// Return complete path to token_id
    /// recurses through parent_token_ids
    GetFullPath { token_id: String },

    /// Analogous to Tokens {}
    /// Returns all paths owned by the given address, [] if unset.
    /// Return type: TokensResponse.
    Paths {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Analogous to Tokens {}
    /// Returns all paths owned by the given address,
    /// where the namespace/parent is token_id
    /// [] if unset.
    /// Return type: TokensResponse.
    PathsForToken {
        owner: String,
        token_id: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Returns token info for a list of token IDs
    /// Includes owner and token metadata
    ListInfoByAlias { aliases: Vec<String> },

    /// Resolve takes a DID identifier and returns the DID
    Resolve { did_id: String },
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

/// Is this a contract? Can it be executed?
/// potentially confusing
/// given the top level Contract Response for the container Contract
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct IsContractResponse {
    pub contract_address: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AddressOfResponse {
    pub owner: String,
    pub contract_address: Option<String>,
    pub validator_address: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct GetParentIdResponse {
    pub parent_token_id: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct GetPathResponse {
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct UserInfo {
    pub alias: String,
    pub owner: String,
    pub metadata: Metadata,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ListUserInfoResponse {
    pub users: Vec<UserInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct DidDocumentResponse {}
