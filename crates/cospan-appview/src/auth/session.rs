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

    /// Retrieve and delete auth flow state (consume it: single use).
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
        let flow = self.auth_flows.write().await.remove(state);

        if let Some(ref f) = flow
            && chrono::Utc::now() > f.expires_at
        {
            tracing::warn!(state = state, "auth flow state expired");
            return Ok(None);
        }

        Ok(flow)
    }
}

/// Redis-backed session store for production.
/// Sessions survive container restarts and deploys.
#[derive(Clone)]
pub struct RedisSessionStore {
    client: redis::Client,
}

impl RedisSessionStore {
    pub fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client })
    }
}

const SESSION_TTL: i64 = 7 * 24 * 3600; // 7 days
const AUTH_FLOW_TTL: i64 = 600; // 10 minutes

#[async_trait]
impl SessionStore for RedisSessionStore {
    async fn put_session(&self, session_id: &str, session: Session) -> anyhow::Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("session:{session_id}");
        let value = serde_json::to_string(&session)?;
        redis::cmd("SET")
            .arg(&key)
            .arg(&value)
            .arg("EX")
            .arg(SESSION_TTL)
            .query_async::<()>(&mut conn)
            .await?;
        Ok(())
    }

    async fn get_session(&self, session_id: &str) -> anyhow::Result<Option<Session>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("session:{session_id}");
        let value: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await?;
        match value {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }

    async fn delete_session(&self, session_id: &str) -> anyhow::Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("session:{session_id}");
        redis::cmd("DEL").arg(&key).query_async::<()>(&mut conn).await?;
        Ok(())
    }

    async fn put_auth_flow(&self, state: &str, flow: AuthFlowState) -> anyhow::Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("authflow:{state}");
        let value = serde_json::to_string(&flow)?;
        redis::cmd("SET")
            .arg(&key)
            .arg(&value)
            .arg("EX")
            .arg(AUTH_FLOW_TTL)
            .query_async::<()>(&mut conn)
            .await?;
        Ok(())
    }

    async fn take_auth_flow(&self, state: &str) -> anyhow::Result<Option<AuthFlowState>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("authflow:{state}");
        // GET then DEL atomically
        let value: Option<String> = redis::cmd("GETDEL")
            .arg(&key)
            .query_async(&mut conn)
            .await?;
        match value {
            Some(v) => {
                let flow: AuthFlowState = serde_json::from_str(&v)?;
                if chrono::Utc::now() > flow.expires_at {
                    tracing::warn!(state = state, "auth flow state expired");
                    return Ok(None);
                }
                Ok(Some(flow))
            }
            None => Ok(None),
        }
    }
}
