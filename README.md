# Whoami

This is an adaptation of the cw-plus onchain metadata contract to allow for listing of usernames on multiple services via NFT metadata.

The main id is of course the minter, but a human-readable username is also required in the meta. This is checked for uniqueness as part of the creation flow.

This change means using the username as a unique key rather than a u8, as in the standard 721-base.

```rust
pub struct Metadata {
    pub username: String, // checked for uniqueness before write
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub twitter_id: Option<String>,
    pub discord_id: Option<String>,
    pub telegram_id: Option<String>,
}
```
