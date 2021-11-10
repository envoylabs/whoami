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

There is also a mapping of `username -> Address` given that `username` is implemented as the value of the `token_id` field on the NFT.
