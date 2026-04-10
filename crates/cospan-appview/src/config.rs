use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub jetstream_url: String,
    pub listen: String,
    pub lexicons_dir: String,
    /// DID of the default Cospan-hosted node used for user-initiated
    /// repo creation and forks when the user has no node of their own.
    pub default_node_did: String,
    /// Public URL of the default Cospan-hosted node.
    pub default_node_url: String,
}

impl AppConfig {
    pub fn public_url(&self) -> String {
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| "https://cospan.dev".to_string())
    }

    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://cospan:cospan@localhost:5432/cospan".into()),
            jetstream_url: std::env::var("JETSTREAM_URL")
                .unwrap_or_else(|_| "wss://jetstream2.us-east.bsky.network/subscribe".into()),
            listen: std::env::var("APPVIEW_LISTEN").unwrap_or_else(|_| "0.0.0.0:3000".into()),
            lexicons_dir: std::env::var("LEXICONS_DIR")
                .unwrap_or_else(|_| "packages/lexicons".into()),
            default_node_did: std::env::var("DEFAULT_NODE_DID").unwrap_or_default(),
            default_node_url: std::env::var("DEFAULT_NODE_URL")
                .unwrap_or_else(|_| "https://node.cospan.dev".into()),
        })
    }
}
