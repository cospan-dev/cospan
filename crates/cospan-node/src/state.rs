use std::sync::Arc;

use tokio::sync::Mutex;

use crate::auth::{AuthzService, SimpleAuthz};
use crate::config::NodeConfig;
use crate::pds_client::{HttpPdsClient, PdsClient};
use crate::store::RepoManager;
use crate::validation::ValidationPipeline;

pub struct NodeState {
    pub config: NodeConfig,
    /// Mutex because FsStore operations are not Send-safe across threads.
    /// All store operations go through spawn_blocking anyway.
    pub store: Arc<Mutex<RepoManager>>,
    pub validator: Arc<ValidationPipeline>,
    pub authz: Arc<dyn AuthzService>,
    pub pds_client: Arc<dyn PdsClient>,
}

impl NodeState {
    pub async fn new(config: NodeConfig) -> anyhow::Result<Self> {
        let repos_dir = config.repos_dir();
        tokio::fs::create_dir_all(&repos_dir).await?;

        let store = Arc::new(Mutex::new(RepoManager::new(repos_dir)));
        let validator = Arc::new(ValidationPipeline::new(&config.validation));
        let authz: Arc<dyn AuthzService> =
            Arc::new(SimpleAuthz::new(config.auth.allowed_dids.clone()));
        let pds_client: Arc<dyn PdsClient> = Arc::new(HttpPdsClient::new());

        Ok(Self {
            config,
            store,
            validator,
            authz,
            pds_client,
        })
    }
}
