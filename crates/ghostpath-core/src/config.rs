use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};
use crate::models::{AppSettings, RoutingRule, ServerNode, Subscription};
use crate::paths::GhostPaths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostConfig {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub settings: AppSettings,
    #[serde(default)]
    pub subscriptions: Vec<Subscription>,
    #[serde(default)]
    pub nodes: Vec<ServerNode>,
    #[serde(default)]
    pub rules: Vec<RoutingRule>,
    #[serde(default)]
    pub active_node_id: Option<String>,
}

impl Default for GhostConfig {
    fn default() -> Self {
        Self {
            version: 1,
            settings: AppSettings::default(),
            subscriptions: Vec::new(),
            nodes: Vec::new(),
            rules: Vec::new(),
            active_node_id: None,
        }
    }
}

impl GhostConfig {
    /// Loads config from standard location (~/.config/ghostpath/config.toml)
    pub fn load() -> Result<Self> {
        let path = GhostPaths::config_file();
        Self::load_from_path(&path)
    }

    /// Loads config from explicit path
    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)
            .map_err(|e| CoreError::Config(format!("Failed to parse TOML: {e}")))?;
        Ok(config)
    }

    /// Saves config to standard location (~/.config/ghostpath/config.toml)
    pub fn save(&self) -> Result<()> {
        let path = GhostPaths::config_file();
        self.save_to_path(&path)
    }

    /// Saves config to explicit path with safe 0600 permissions
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| CoreError::Config(format!("Failed to serialize TOML: {e}")))?;
        fs::write(path, toml_str)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(path, permissions);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = GhostConfig::default();
        config.settings.mtu = 1380;
        config.settings.socks_port = 10808;

        config.save_to_path(&path).unwrap();
        assert!(path.exists());

        let loaded = GhostConfig::load_from_path(&path).unwrap();
        assert_eq!(loaded.settings.mtu, 1380);
        assert_eq!(loaded.settings.socks_port, 10808);
    }
}
