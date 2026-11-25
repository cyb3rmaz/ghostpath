use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::crypto::{decrypt_secret, encrypt_secret, key_path_for_config, load_or_create_key};
use crate::error::{CoreError, Result};
use crate::fs_safe::{atomic_write_private, read_to_string_no_follow};
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

    /// Loads config from explicit path (refuses symlinks and caps size at 16 MiB)
    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = read_to_string_no_follow(path, 16 * 1024 * 1024)?;
        let mut config: Self = toml::from_str(&content)
            .map_err(|e| CoreError::Config(format!("Failed to parse TOML: {e}")))?;

        let key_path = key_path_for_config(path);
        if key_path.exists() {
            let key = load_or_create_key(&key_path)?;
            for node in &mut config.nodes {
                if let Some(ref raw) = node.raw_link {
                    node.raw_link = Some(decrypt_secret(raw, &key)?);
                }
            }
            for sub in &mut config.subscriptions {
                sub.url = decrypt_secret(&sub.url, &key)?;
            }
        }
        Ok(config)
    }

    /// Saves config to standard location (~/.config/ghostpath/config.toml)
    pub fn save(&self) -> Result<()> {
        let path = GhostPaths::config_file();
        self.save_to_path(&path)
    }

    /// Saves config atomically with `0700` dir and `0600` file permissions, encrypting
    /// `raw_link` and subscription URLs at rest with `ChaCha20-Poly1305` and rejecting symlinks.
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        let mut sealed = self.clone();
        let has_secrets =
            sealed.nodes.iter().any(|n| n.raw_link.is_some()) || !sealed.subscriptions.is_empty();
        if has_secrets {
            let key_path = key_path_for_config(path);
            let key = load_or_create_key(&key_path)?;
            for node in &mut sealed.nodes {
                if let Some(ref raw) = node.raw_link {
                    node.raw_link = Some(encrypt_secret(raw, &key)?);
                }
            }
            for sub in &mut sealed.subscriptions {
                sub.url = encrypt_secret(&sub.url, &key)?;
            }
        }
        let toml_str = toml::to_string_pretty(&sealed)
            .map_err(|e| CoreError::Config(format!("Failed to serialize TOML: {e}")))?;
        atomic_write_private(path, toml_str.as_bytes())?;
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

    #[test]
    fn test_config_encrypts_secrets_at_rest() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = GhostConfig::default();
        config.nodes.push(ServerNode {
            id: "node-1".to_string(),
            name: "Node 1".to_string(),
            country_code: "DE".to_string(),
            city: "Frankfurt".to_string(),
            country: "Germany".to_string(),
            protocol: crate::models::NodeProtocol::VLESS,
            transport: "tcp".to_string(),
            security: "reality".to_string(),
            host: "198.51.100.1".to_string(),
            port: 443,
            latency_ms: None,
            pinging: None,
            load_percent: 0,
            subscription_id: None,
            raw_link: Some(
                "vless://super-secret-uuid@198.51.100.1:443?pbk=realitykey#N1".to_string(),
            ),
        });
        config.subscriptions.push(Subscription {
            id: "sub-1".to_string(),
            name: "Sub 1".to_string(),
            url: "https://provider.example/sub?token=secret-sub-token".to_string(),
            used_gb: 0.0,
            total_gb: 100.0,
            expires_at: "2027-01-01".to_string(),
            last_updated: "now".to_string(),
            auto_update_hours: 12,
            updating: None,
        });

        config.save_to_path(&path).unwrap();

        let disk_toml = std::fs::read_to_string(&path).unwrap();
        assert!(disk_toml.contains("enc:v1:"));
        assert!(!disk_toml.contains("super-secret-uuid"));
        assert!(!disk_toml.contains("realitykey"));
        assert!(!disk_toml.contains("secret-sub-token"));

        let loaded = GhostConfig::load_from_path(&path).unwrap();
        assert_eq!(
            loaded.nodes[0].raw_link.as_deref(),
            Some("vless://super-secret-uuid@198.51.100.1:443?pbk=realitykey#N1")
        );
        assert_eq!(
            loaded.subscriptions[0].url,
            "https://provider.example/sub?token=secret-sub-token"
        );
    }
}
