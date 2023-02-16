use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// This is the service that this contract will be exposed via
/// it should be set on creation of the contract
/// question: should it be updateable?
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Service {
    /// This id should be (for example) the instantiated address of either this
    /// contract (if totally self-contained), or a calling contract if that is
    /// simply using this one to create DID documents
    pub id: String,
    /// Service type
    pub r#type: String,
    /// The endpoint to access this service
    /// this could conceivably be an endpoint on a domain
    /// or the smart contract itself
    /// so in this case, the DID contract address would be the service_endpoint
    /// and the address of the calling contract creating DIDs would be the ID
    #[serde(rename = "serviceEndpoint")]
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
    #[serde(rename = "publicKeyBase58")]
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
    #[serde(rename = "publicKeyPgp")]
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
    #[serde(rename = "blockchainAccountId")]
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

/// This models a associative data structure defined in the spec
/// where a DID is followed by a verification method
/// so they're sort of pairs for some reason but in JSON
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum DidToVerificationMapping {
    String,
    VerificationMethod,
}

/// The DID Document
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct DidDocument {
    /// The LD context. Will always be "https://www.w3.org/ns/did/v1"
    #[serde(rename = "@context")]
    pub context: String,
    /// This will be the did id
    /// it is auto-generated
    pub id: String,
    /// Controller
    /// Note that for our case in this first implementation
    /// we always insist that the controller
    /// is the same thing as the subject
    /// another way of approaching this would be to make the controller
    /// the smart contract itself, or even a DAO
    pub controller: Vec<String>,
    /// Verification Method
    /// at the very least, BlockchainAccountId is required
    /// this is an Option
    /// but in the implementation we will insist on it
    /// this does however mean it can be updated & removed
    #[serde(rename = "verificationMethod")]
    pub verification_method: Option<Vec<VerificationMethod>>,
    /// Services.
    /// at least one should be set
    /// when the contract is instantiated
    /// so it is optional in name only
    /// purely cos spec says so YOLO
    pub service: Option<Vec<Service>>,
    #[serde(rename = "assertionMethod")]
    pub assertion_method: Option<Vec<DidToVerificationMapping>>,
    #[serde(rename = "keyAgreement")]
    pub key_agreement: Option<Vec<DidToVerificationMapping>>,
    #[serde(rename = "capabilityInvocation")]
    pub capability_invocation: Option<Vec<DidToVerificationMapping>>,
    #[serde(rename = "capabilityDelegation")]
    pub capability_delegation: Option<Vec<DidToVerificationMapping>>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum DidExecuteMsg {
    /// DID Method: Create
    Create {
        id: String,
    },

    /// DID Method: Update
    Update {
        id: String,
    },

    /// adding service
    AddService {
        id: String,
        service: Service,
    },
    DeleteService {
        id: String,
        service_id: String,
    },

    /// DID Method: Deactivate
    Delete {
        id: String,
    },

    /// DID Method: Create, executed by a permitted proxy.
    /// it is assumed that this will be a contract
    ProxyCreate {
        /// this is the wrapped message
        create_msg: ProxyCreateWrappedMsg,
    },
}

/// Let's keep this simple for first impl
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ProxyCreateWrappedMsg {
    id: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum DidQueryMsg {
    /// Resolve takes a DID identifier and returns the DID
    Resolve { id: String },

    /// DID Method: Read
    /// returns only the DID document, not the NFT
    Read { did: String },
}

/// This is the same as a DID document but the authentication field
/// potentially contains duplicate data of the verification methods
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct DidDocumentResponse {
    pub did_document: Option<DidDocument>,
}

// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
// pub enum DidArgs {

// }
