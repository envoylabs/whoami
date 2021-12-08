# Whoami

This is an adaptation of the cw-nfts onchain metadata contract to
allow for listing of usernames on multiple services via NFT metadata.

```rust
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub email: Option<String>,
    pub external_url: Option<String>,
    pub public_name: Option<String>,
    pub public_bio: Option<String>,
    pub twitter_id: Option<String>,
    pub discord_id: Option<String>,
    pub telegram_id: Option<String>,
    pub keybase_id: Option<String>,
    pub validator_operator_address: Option<String>,
}
```

## Dev quickstart

Run a juno node in docker, using the default docker compose file. Its
node name should be `juno_node_1`.

It will log an address as it starts, copy this: `juno10j9...` so that
it can be used to init the contract:

```
bash scripts/deploy_local.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg
```

To use the account configured by the deploy script import the account
in `default-account.txt` into your keplr wallet.

## Mapping address -> username

There is an additional query message that allows for an owner set
alias to be returned.

```rust
PreferredAlias { address: String }
```

This returns:

```rust
// returns a token_id (i.e. a username)
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PreferredAliasResponse {
    pub username: String,
}
```

Its default behaviour is to return the last NFT in the list owned by
the address (LILO). Alternatively, the user can set a preferred alias.

### Setting a preferred alias

An owner might have multiple NFTs.

Setting a preferred alias is done via a new `ExecuteMsg` variant. On
`burn`, `transfer_nft` or `send_nft`, this entry will be cleared from
storage.

```rust
UpdatePreferredAlias {
    token_id: String,
},
```

### Other query strategies

It is possible also to use `token_info` and pass in a limit of 1, to
match the default behaviour of the `PreferredAlias` query message.

```rust
Tokens {
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
}
```

## Mapping username -> address

TL;DR - use `owner_of`.

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
