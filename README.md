# Whoami

This is an adaptation of the cw-nfts onchain metadata contract to allow for listing of usernames on multiple services via NFT metadata.

```rust
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
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

## Mapping username -> address

TL;DR - use `owner_of`.

```rust
OwnerOf {
    token_id: String,
    include_expired: Option<bool>,
},
```

The mapping of `username -> address` is in practice simply the link between `token_id` (the string username) and the `owner`. As/when the username is transferred or sold, this is updated with no additional computation required.

## Mapping address -> username

TL;DR - use `token_info`.

```rust
Tokens {
    owner: String,
    start_after: Option<String>,
    limit: Option<u32>,
}
```

If we pass `1` into `limit`, we get a mapping of address (`owner`) to username (NFT).

In future we could do one of several things;

1. Stop addresses having more than one NFT (suggest initially restricting this via the UI) by changing the multi-index type to a normal hashmap
2. Maintain a secondary index of preferred mapping, and update this (however this involves a lot of extension)
3. Encourage other applications to let a user store their preferred NFT alias after listing all Whoami NFTs owned by that address
