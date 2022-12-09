use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    #[serde(rename = "@context")]
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

pub enum DidExecuteMsg {
    /// DID Method: Create
    /// top level Metadata fields are automatically set;
    /// this means that in practice
    /// Create can be called and only pass in DID data.
    /// Under the hood it is using mint
    Create { id: String },

    /// DID Method: Update
    /// this is similar, but not identical
    /// to UpdateMetadata
    /// as it only operates on the DID document
    Update { id: String },

    /// DID Method: Deactivate
    /// this is mapped to Burn
    Delete { id: String },
}

pub enum DidQueryMsg {
    /// Resolve takes a DID identifier and returns the DID
    Resolve { id: String },

    /// DID Method: Read
    /// returns only the DID document, not the NFT
    Read { id: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct DidDocumentResponse {}
