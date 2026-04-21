use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct NodeConfig {
    pub did: String,
    pub listen: String,
    pub data_dir: PathBuf,
    #[serde(default)]
    pub validation: ValidationConfig,
    #[serde(default)]
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidationConfig {
    #[serde(default = "default_check_mode")]
    pub gat_type_check: CheckMode,
    #[serde(default = "default_check_mode")]
    pub equation_verify: CheckMode,
    #[serde(default = "default_check_mode")]
    pub breaking_change: CheckMode,
    #[serde(default)]
    pub auto_lens: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CheckMode {
    Strict,
    Warn,
    #[default]
    Skip,
}

fn default_check_mode() -> CheckMode {
    CheckMode::Skip
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            gat_type_check: CheckMode::Skip,
            equation_verify: CheckMode::Skip,
            breaking_change: CheckMode::Skip,
            auto_lens: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuthConfig {
    #[serde(default)]
    pub allowed_dids: Vec<String>,
    /// URL of the appview's JWKS endpoint for verifying push tokens.
    /// e.g. "https://cospan.dev/.well-known/jwks.json"
    #[serde(default)]
    pub appview_jwks_url: Option<String>,
}

impl NodeConfig {
    pub fn load() -> anyhow::Result<Self> {
        // Try config file first, fall back to env
        let config_path = std::env::var("COSPAN_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".cospan")
                    .join("config.toml")
            });

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: NodeConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Fall back to env vars for development
            Ok(NodeConfig {
                did: std::env::var("NODE_DID").unwrap_or_else(|_| "did:plc:dev".to_string()),
                listen: std::env::var("NODE_LISTEN").unwrap_or_else(|_| "0.0.0.0:3001".to_string()),
                data_dir: std::env::var("COSPAN_DATA_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| {
                        dirs::home_dir()
                            .unwrap_or_else(|| PathBuf::from("."))
                            .join(".cospan")
                    }),
                validation: ValidationConfig::default(),
                auth: AuthConfig {
                    appview_jwks_url: std::env::var("APPVIEW_JWKS_URL").ok(),
                    ..Default::default()
                },
            })
        }
    }

    pub fn repos_dir(&self) -> PathBuf {
        self.data_dir.join("repos")
    }
}
