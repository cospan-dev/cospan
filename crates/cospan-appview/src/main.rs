use std::sync::Arc;

use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use cospan_appview::auth::OAuthConfig;
use cospan_appview::auth::dpop::DpopKey;
use cospan_appview::auth::session::{InMemorySessionStore, RedisSessionStore, SessionStore};
use cospan_appview::config::AppConfig;
use cospan_appview::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cospan_appview=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    dotenvy::dotenv().ok();
    let config = AppConfig::from_env()?;
    let oauth_config = OAuthConfig::from_env()?;

    tracing::info!(listen = %config.listen, "starting cospan-appview");
    tracing::info!(client_id = %oauth_config.client_id, "OAuth configured");

    // Connect to PostgreSQL
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("database migrations applied");

    // Backfill: if BACKFILL_HOURS is set, reset the cursor to N hours ago
    // so the indexer replays Jetstream history (up to ~72h retained)
    if let Some(h) = std::env::var("BACKFILL_HOURS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
    {
        let now_us = chrono::Utc::now().timestamp_micros();
        let backfill_us = now_us - (h * 3600 * 1_000_000);
        cospan_appview::db::cursor::save_cursor(&pool, backfill_us).await?;
        tracing::info!(
            hours = h,
            cursor_us = backfill_us,
            "backfill: cursor reset to {h} hours ago"
        );
    }

    // Initialize auth infrastructure
    let dpop_key = DpopKey::generate();
    tracing::info!(kid = %dpop_key.kid, "DPoP signing key generated");

    let session_store: Arc<dyn SessionStore> =
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            let store = RedisSessionStore::new(&redis_url)?;
            tracing::info!("using Redis session store");
            Arc::new(store)
        } else {
            tracing::warn!("REDIS_URL not set, using in-memory session store (sessions lost on restart)");
            Arc::new(InMemorySessionStore::new())
        };

    let state =
        Arc::new(AppState::new(config.clone(), pool, oauth_config, session_store, dpop_key).await?);

    // Spawn the firehose indexer
    let indexer_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = cospan_appview::indexer::run(indexer_state).await {
            tracing::error!(error = %e, "indexer crashed");
        }
    });

    // Build and start the HTTP server
    let app = cospan_appview::xrpc::router(state);
    let listener = tokio::net::TcpListener::bind(&config.listen).await?;
    tracing::info!("listening on {}", config.listen);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
