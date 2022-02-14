# Whoami

This is an adaptation of the cw-nfts onchain metadata contract to
allow for listing of usernames on multiple services via NFT metadata.

```rust
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<Logo>, // see cw-plus CW20
    pub email: Option<String>,
    pub external_url: Option<String>,
    pub public_name: Option<String>,
    pub public_bio: Option<String>,
    pub twitter_id: Option<String>,
    pub discord_id: Option<String>,
    pub telegram_id: Option<String>,
    pub keybase_id: Option<String>,
    pub validator_operator_address: Option<String>,
    pub contract_address: Option<String>, // marks this as executable
    pub parent_token_id: Option<String>, // for paths/nested tokens
    pub pgp_public_key: Option<String>, // exactly what you think it is
}
```

### Minting

Minting is governed by a fee:

- Base fee: charged for every mint (optional, and updateable)
- Surcharge: charged for short usernames (optional, configurable and updateable)

These two added together forms the `mint_fee`. This is divided between:

1. The admin address (which could point to a multisig or DAO)
2. Burning

The burn percentage is configured at instantiation time.

## Dev quickstart

Bootstrap the project like so:

```bash
./scripts/deploy_local.sh juno16g2rahf5846rxzp3fwlswy08fz8ccuwk03k57y
```

To use the account configured by the deploy script import the account
in `./scripts/test-user.env` into your keplr wallet.

## Mapping address -> username

There is an additional query message that allows for an owner set
alias to be returned.

This will return `StdError::NotFound` if there is no Primary Alias.
Generally speaking, this error condition will only be hit if a user has only Paths, but no Base Tokens.
This is an edge case that can happen if a user has had a token transferred to them (e.g. after a sale).

```rust
PrimaryAlias { address: String }
```

This returns:

```rust
// returns a token_id (i.e. a username)
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PrimaryAliasResponse {
    pub username: String,
}
```

Its default behaviour is to return the last NFT in the list owned by
the address (LILO). Alternatively, the user can set a primary alias.

### Setting a primary alias

An owner might have multiple NFTs.

Setting a primary alias is done via a new `ExecuteMsg` variant. On
`burn`, `transfer_nft` or `send_nft`, this entry will be cleared from
storage.

```rust
UpdatePrimaryAlias {
    token_id: String,
},
```

### Other query strategies

It is possible also to use `token_info` and pass in a limit of 1, to
match the default behaviour of the `PrimaryAlias` query message.

```rust
Tokens {
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
}
```

## Mapping username -> address

Use `AddressOf` to get the address linked to a token. This can be used as an alias for `OwnerOf` with fewer requirements.

Crucially, however, it also returns `contract_address` and `validator_address` from the meta fields. In some cases, it's likely that you actually want either the owner's address, _or_ a `contract_address`, if found.

```rust
AddressOf {
    token_id: String,
    contract_address: Option<String>,
    validator_address: Option<String>,
},
```

If you're a fan of just using the NFT implementation - use `owner_of`. This also allows you to specify expiry.

```rust
OwnerOf {
    token_id: String,
    include_expired: Option<bool>,
},
```

The mapping of `username -> address` is in practice simply the link
between `token_id` (the string username) and the `owner`. As/when the
username is transferred or sold, this is updated with no additional
computation required.

## Paths

Although it is complex to do UI for, and most users don't seem to need it, subdomains are supported in this first version. We call them Paths.

It is important to note that while minting is governed by a fee, Paths can be minted for free (other than gas etc) by the owner of the Base Token.

- Minting is done via `MintPath`
    - When minting, _only pass in the token_id you want to mint as a Path_. The parent is automatically resolved and prepended by the contract.
    - i.e. for `jeffvader` as `parent_token_id` pass in `projects` as a `token_id` to `mint_path`. You will get back `jeffvader::projects`.
- Base name tokens that are _not_ Paths can be queried with `BaseTokens`
- Paths (and not Base tokens) can be queried with `Paths`
- Paths nested under a token can be queried with `PathsForToken`

### Getting a full path

For resolving a full path, selecting a parent, or working with subdomains, you will want to resolve the tree of `parent_token_id`s that a token has.

The query for this is `GetFullPath`:

```rust
GetFullPath { token_id: String }
```

This returns:

```rust
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct GetPathResponse {
    pub path: String,
}
```

Where `path` will likely be `something::like::this` made up of nested tokens delimited by slashes.
Although it is _very_ unlikely people will do so other than as a flex, normal tokens can be nested.
They are delineated by `/`, such that `jeffvader/vehicles::tie-fighter` is:

- `jeffvader` (Base Token)
- `vehicles` (Base Token)
- `vehicles::tie-fighter` (Path)

Note that only these three are resolvable without any recursive lookups. `jeffvader/vehicles::tie-fighter` would require the UI to recurse, as it is a compound name.

This is why in general, UIs should probably _not_ allow Base Tokens to be nested, unless there's some amazing use case we've not thought of.

Make sense? Cool.

### Getting the Token ID for a parent

With the following query, you pass in a token's ID, and get back the ID of their parent, if it exists.

Will return `StdError::NotFound` if there is no parent.

```rust
GetParentId {
    token_id: String,
}
```

### Getting the NFT info for a parent

With this, you return the `NftInfoResponse` for a parent.

Will return `StdError::NotFound` if there is no parent.

```rust
GetParentInfo {
    token_id: String,
}
```
