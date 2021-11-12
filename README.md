# Whoami

This is an adaptation of the cw-plus onchain metadata contract to allow for listing of usernames on multiple services via NFT metadata.

The main id is of course the minter, but a human-readable username is also required in the meta. This is checked for uniqueness as part of the creation flow.

```rust
pub struct Metadata {
    pub username: String, // checked for uniqueness before write
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub twitter_id: Option<String>,
    pub discord_id: Option<String>,
    pub telegram_id: Option<String>,
    pub keybase_id: Option<String>,
}
```

The mapping of `username -> address` is in practice simply the link between `token_id` (the string username) and the `owner`. As/when the username is transferred or sold, this is updated with no additional computation required.
