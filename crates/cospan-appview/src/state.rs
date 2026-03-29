use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::auth::OAuthConfig;
use crate::auth::did_resolver::DidResolver;
use crate::auth::dpop::DpopKey;
use crate::auth::session::SessionStore;
use crate::config::AppConfig;
use crate::interop::TangledInterop;
use crate::xrpc::sse::IndexEvent;

/// Channel capacity for the event bus. Events are dropped if all receivers lag.
const EVENT_BUS_CAPACITY: usize = 4096;

pub struct AppState {
    pub config: AppConfig,
    pub db: PgPool,
    pub oauth_config: OAuthConfig,
    pub session_store: Arc<dyn SessionStore>,
    pub did_resolver: Arc<DidResolver>,
    pub dpop_key: Arc<DpopKey>,
    pub http_client: reqwest::Client,
    /// Broadcast channel for streaming index events to SSE clients.
    pub event_tx: broadcast::Sender<IndexEvent>,
    /// Tangled → Cospan interop morphisms (compiled at startup).
    pub tangled_interop: TangledInterop,
}

impl AppState {
    pub async fn new(
        config: AppConfig,
        db: PgPool,
        oauth_config: OAuthConfig,
        session_store: Arc<dyn SessionStore>,
        dpop_key: DpopKey,
    ) -> anyhow::Result<Self> {
        let http_client = reqwest::Client::builder()
            .user_agent("Cospan/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let did_resolver = Arc::new(DidResolver::new(http_client.clone()));

        let (event_tx, _) = broadcast::channel(EVENT_BUS_CAPACITY);

        // Load Tangled→Cospan interop morphisms from compiled codegen output.
        let lexicons_dir = std::path::PathBuf::from(&config.lexicons_dir);
        let tangled_interop = TangledInterop::load(&lexicons_dir)
            .expect("failed to load tangled interop morphisms — run `cargo run -p cospan-codegen` first");

        Ok(Self {
            config,
            db,
            oauth_config,
            session_store,
            did_resolver,
            dpop_key: Arc::new(dpop_key),
            http_client,
            event_tx,
            tangled_interop,
        })
    }
}
