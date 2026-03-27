use crate::error::NodeError;

/// Authorization service for checking permissions.
pub trait AuthzService: Send + Sync {
    fn check_push(&self, did: &str, repo_did: &str, repo_name: &str) -> Result<(), NodeError>;
}

/// Simple authorization: allow specific DIDs or allow all if list is empty.
pub struct SimpleAuthz {
    allowed_dids: Vec<String>,
}

impl SimpleAuthz {
    pub fn new(allowed_dids: Vec<String>) -> Self {
        Self { allowed_dids }
    }
}

impl AuthzService for SimpleAuthz {
    fn check_push(&self, did: &str, _repo_did: &str, _repo_name: &str) -> Result<(), NodeError> {
        if self.allowed_dids.is_empty() || self.allowed_dids.contains(&did.to_string()) {
            Ok(())
        } else {
            Err(NodeError::Forbidden(format!(
                "{did} is not authorized to push"
            )))
        }
    }
}
