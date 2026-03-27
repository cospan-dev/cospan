use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::{AuthFlowState, Session};

/// Trait for storing user sessions and temporary auth flow state.
#[async_trait]
pub trait SessionStore: Send + Sync + 'static {
    /// Store a session keyed by session ID.
    async fn put_session(&self, session_id: &str, session: Session) -> anyhow::Result<()>;

    /// Retrieve a session by session ID.
    async fn get_session(&self, session_id: &str) -> anyhow::Result<Option<Session>>;

    /// Delete a session by session ID.
    async fn delete_session(&self, session_id: &str) -> anyhow::Result<()>;

    /// Store temporary auth flow state keyed by the `state` parameter.
    async fn put_auth_flow(&self, state: &str, flow: AuthFlowState) -> anyhow::Result<()>;

    /// Retrieve and delete auth flow state (consume it — single use).
    async fn take_auth_flow(&self, state: &str) -> anyhow::Result<Option<AuthFlowState>>;
}

/// In-memory session store for development. Not suitable for production
/// (no persistence, no distribution across instances).
#[derive(Debug, Clone)]
pub struct InMemorySessionStore {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    auth_flows: Arc<RwLock<HashMap<String, AuthFlowState>>>,
}

impl InMemorySessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            auth_flows: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn put_session(&self, session_id: &str, session: Session) -> anyhow::Result<()> {
        self.sessions
            .write()
            .await
            .insert(session_id.to_string(), session);
        Ok(())
    }

    async fn get_session(&self, session_id: &str) -> anyhow::Result<Option<Session>> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id).cloned();
        // Check expiration: if the access token has expired, the session still exists
        // but the caller should trigger a refresh. We don't auto-delete here.
        Ok(session)
    }

    async fn delete_session(&self, session_id: &str) -> anyhow::Result<()> {
        self.sessions.write().await.remove(session_id);
        Ok(())
    }

    async fn put_auth_flow(&self, state: &str, flow: AuthFlowState) -> anyhow::Result<()> {
        self.auth_flows
            .write()
            .await
            .insert(state.to_string(), flow);
        Ok(())
    }

    async fn take_auth_flow(&self, state: &str) -> anyhow::Result<Option<AuthFlowState>> {
        // Remove and return (single use)
        let flow = self.auth_flows.write().await.remove(state);

        // Check expiry
        if let Some(ref f) = flow
            && chrono::Utc::now() > f.expires_at
        {
            tracing::warn!(state = state, "auth flow state expired");
            return Ok(None);
        }

        Ok(flow)
    }
}
