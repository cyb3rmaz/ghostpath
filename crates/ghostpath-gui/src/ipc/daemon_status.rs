use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonStateKind {
    Running,
    NotRunning,
    NotInstalled,
    PermissionDenied,
    VersionMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonHealth {
    pub state: DaemonStateKind,
    pub message: String,
    pub fix_command: Option<String>,
    pub details: Option<String>,
}

impl DaemonHealth {
    pub fn running(version: &str) -> Self {
        Self {
            state: DaemonStateKind::Running,
            message: format!("GhostPath daemon is running (v{})", version),
            fix_command: None,
            details: None,
        }
    }

    pub fn not_running(details: Option<String>) -> Self {
        Self {
            state: DaemonStateKind::NotRunning,
            message: "GhostPath background service (ghostpathd) is not running.".to_string(),
            fix_command: Some("sudo systemctl start ghostpathd".to_string()),
            details,
        }
    }

    pub fn not_installed() -> Self {
        Self {
            state: DaemonStateKind::NotInstalled,
            message: "GhostPath daemon (ghostpathd) is not installed on this system.".to_string(),
            fix_command: Some("sudo apt install ghostpath # or sudo dnf install ghostpath".to_string()),
            details: Some("The GUI requires the privileged system daemon to manage network interfaces and routes.".to_string()),
        }
    }

    pub fn permission_denied() -> Self {
        Self {
            state: DaemonStateKind::PermissionDenied,
            message: "Access denied to GhostPath IPC socket.".to_string(),
            fix_command: Some("sudo usermod -aG ghostpath $USER && newgrp ghostpath".to_string()),
            details: Some("Your current user account must belong to the 'ghostpath' group to communicate with the daemon.".to_string()),
        }
    }

    pub fn version_mismatch(client_ver: &str, daemon_ver: &str) -> Self {
        Self {
            state: DaemonStateKind::VersionMismatch,
            message: format!("Version mismatch: GUI is v{}, daemon is v{}.", client_ver, daemon_ver),
            fix_command: Some("sudo systemctl restart ghostpathd".to_string()),
            details: Some("Restart the daemon service to reload matching binaries.".to_string()),
        }
    }

    pub fn inspect_offline(socket_path: &Path, daemon_bin_path: Option<&Path>) -> Self {
        let bin_exists = daemon_bin_path
            .map(|p| p.exists())
            .unwrap_or_else(|| {
                Path::new("/usr/sbin/ghostpathd").exists()
                    || Path::new("/usr/bin/ghostpathd").exists()
                    || Path::new("/usr/local/bin/ghostpathd").exists()
            });

        if !socket_path.exists() {
            if !bin_exists {
                Self::not_installed()
            } else {
                Self::not_running(Some("Socket file does not exist in /run/ghostpath/".to_string()))
            }
        } else {
            // Socket file exists but connection might fail or be refused
            Self::not_running(Some("Socket exists but daemon is not responding.".to_string()))
        }
    }
}

pub async fn check_daemon_health(socket_path: &Path, expected_version: &str) -> DaemonHealth {
    #[cfg(unix)]
    {
        use std::time::Duration;
        use tokio::net::UnixStream;

        if !socket_path.exists() {
            return DaemonHealth::inspect_offline(socket_path, None);
        }

        match tokio::time::timeout(Duration::from_millis(500), UnixStream::connect(socket_path)).await {
            Ok(Ok(_stream)) => {
                // Connected successfully
                DaemonHealth::running(expected_version)
            }
            Ok(Err(e)) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    DaemonHealth::permission_denied()
                } else if e.kind() == std::io::ErrorKind::ConnectionRefused {
                    DaemonHealth::not_running(Some("Stale socket file detected (connection refused).".to_string()))
                } else {
                    DaemonHealth::not_running(Some(format!("Connection error: {}", e)))
                }
            }
            Err(_) => {
                DaemonHealth::not_running(Some("Connection timed out.".to_string()))
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = expected_version;
        DaemonHealth::inspect_offline(socket_path, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_daemon_health_not_installed() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("ghostpathd.sock");
        let non_existent_bin = dir.path().join("ghostpathd_missing");

        let health = DaemonHealth::inspect_offline(&sock, Some(&non_existent_bin));
        assert_eq!(health.state, DaemonStateKind::NotInstalled);
        assert!(health.fix_command.unwrap().contains("install"));
    }

    #[test]
    fn test_daemon_health_not_running() {
        let dir = tempdir().unwrap();
        let sock = dir.path().join("ghostpathd.sock");
        let fake_bin = dir.path().join("ghostpathd");
        std::fs::write(&fake_bin, b"binary").unwrap();

        let health = DaemonHealth::inspect_offline(&sock, Some(&fake_bin));
        assert_eq!(health.state, DaemonStateKind::NotRunning);
        assert!(health.fix_command.unwrap().contains("systemctl start"));
    }

    #[test]
    fn test_daemon_health_permission_denied() {
        let health = DaemonHealth::permission_denied();
        assert_eq!(health.state, DaemonStateKind::PermissionDenied);
        assert!(health.fix_command.unwrap().contains("ghostpath"));
    }

    #[test]
    fn test_daemon_health_version_mismatch() {
        let health = DaemonHealth::version_mismatch("0.2.0", "0.1.0");
        assert_eq!(health.state, DaemonStateKind::VersionMismatch);
        assert!(health.message.contains("0.2.0"));
    }
}
