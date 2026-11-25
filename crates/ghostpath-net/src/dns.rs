use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ghostpath_core::error::Result;
use ghostpath_core::fs_safe::{atomic_write_private, read_to_string_no_follow};
use ghostpath_core::paths::GhostPaths;
use ghostpath_core::validators::{validate_dns_endpoint, validate_iface_name};

use crate::journal::{JournalAction, UndoJournal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsStrategy {
    SystemdResolved,
    NetworkManager,
    DirectResolvConf,
    Mock,
}

pub struct DnsManager {
    strategy: DnsStrategy,
    primary_dns: String,
    fallback_dns: String,
    backup_path: PathBuf,
    dry_run: bool,
}

impl DnsManager {
    pub fn new(
        primary_dns: impl Into<String>,
        fallback_dns: impl Into<String>,
        dry_run: bool,
    ) -> Self {
        let strategy = detect_dns_strategy();
        Self {
            strategy,
            primary_dns: primary_dns.into(),
            fallback_dns: fallback_dns.into(),
            backup_path: GhostPaths::resolv_conf_backup(),
            dry_run,
        }
    }

    pub fn strategy(&self) -> DnsStrategy {
        self.strategy
    }

    /// Sets up DNS routing so queries go exclusively through `dev`
    pub fn apply_dns(&mut self, dev: &str, journal: &mut UndoJournal) -> Result<DnsGuard> {
        validate_iface_name(dev)?;
        validate_dns_endpoint(&self.primary_dns)?;
        validate_dns_endpoint(&self.fallback_dns)?;
        tracing::info!(
            strategy = ?self.strategy,
            primary = %self.primary_dns,
            fallback = %self.fallback_dns,
            dev = dev,
            "Applying GhostPath DNS configuration"
        );

        match self.strategy {
            DnsStrategy::SystemdResolved => {
                self.apply_resolved_dns(dev)?;
                journal.record(JournalAction::ModifiedDns {
                    original_backup_path: "systemd-resolved".to_string(),
                    method: "resolved-dbus".to_string(),
                });
            }
            DnsStrategy::DirectResolvConf | DnsStrategy::NetworkManager => {
                self.apply_resolv_conf_dns(journal)?;
            }
            DnsStrategy::Mock => {
                tracing::info!("[mock] Applied mock DNS configuration");
            }
        }

        Ok(DnsGuard {
            active: true,
            dev: dev.to_string(),
            strategy: self.strategy,
            backup_path: self.backup_path.clone(),
            dry_run: self.dry_run,
        })
    }

    fn apply_resolved_dns(&self, dev: &str) -> Result<()> {
        if self.dry_run {
            tracing::info!(dev = dev, primary = %self.primary_dns, "[dry-run] resolvectl dns & domain ~.");
            return Ok(());
        }

        // Set DNS servers on the link
        let output = Command::new("resolvectl")
            .args(["dns", dev, &self.primary_dns, &self.fallback_dns])
            .output();

        if let Ok(out) = output {
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr);
                tracing::warn!(error = %err, "resolvectl dns failed, falling back to resolv.conf");
                return self.backup_and_write_resolv_conf();
            }
        } else {
            return self.backup_and_write_resolv_conf();
        }

        // Set routing domain "~." to make ghost0 the preferred/exclusive resolver for all queries
        let dom_out = Command::new("resolvectl")
            .args(["domain", dev, "~."])
            .output();

        if let Ok(out) = dom_out {
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr);
                tracing::warn!(error = %err, "resolvectl domain ~. failed");
            }
        }

        // Set default-route true on link
        let _ = Command::new("resolvectl")
            .args(["default-route", dev, "true"])
            .output();

        tracing::info!(
            dev = dev,
            "systemd-resolved successfully configured with exclusive routing domain (~.)"
        );
        Ok(())
    }

    fn apply_resolv_conf_dns(&self, journal: &mut UndoJournal) -> Result<()> {
        if self.dry_run {
            tracing::info!(primary = %self.primary_dns, "[dry-run] /etc/resolv.conf updated");
            journal.record(JournalAction::ModifiedDns {
                original_backup_path: self.backup_path.to_string_lossy().to_string(),
                method: "resolv.conf-backup".to_string(),
            });
            return Ok(());
        }

        self.backup_and_write_resolv_conf()?;
        journal.record(JournalAction::ModifiedDns {
            original_backup_path: self.backup_path.to_string_lossy().to_string(),
            method: "resolv.conf-backup".to_string(),
        });
        Ok(())
    }

    fn backup_and_write_resolv_conf(&self) -> Result<()> {
        let resolv_conf = Path::new("/etc/resolv.conf");
        if resolv_conf.exists() && !self.backup_path.exists() {
            if let Ok(existing) = read_to_string_no_follow(resolv_conf, 64 * 1024) {
                let _ = atomic_write_private(&self.backup_path, existing.as_bytes());
            }
        }

        let new_content = format!(
            "# Generated by GhostPath VPN\nnameserver {}\nnameserver {}\noptions edns0 trust-ad\n",
            self.primary_dns, self.fallback_dns
        );

        atomic_write_private(resolv_conf, new_content.as_bytes())?;
        Ok(())
    }

    pub fn restore_dns(&self, dev: &str) -> Result<()> {
        tracing::info!(dev = dev, strategy = ?self.strategy, "Restoring original system DNS settings");

        if self.dry_run {
            return Ok(());
        }

        match self.strategy {
            DnsStrategy::SystemdResolved => {
                let _ = Command::new("resolvectl").args(["revert", dev]).output();
            }
            DnsStrategy::DirectResolvConf | DnsStrategy::NetworkManager => {
                let resolv_conf = Path::new("/etc/resolv.conf");
                if self.backup_path.exists() {
                    let _ = fs::copy(&self.backup_path, resolv_conf);
                    let _ = fs::remove_file(&self.backup_path);
                }
            }
            DnsStrategy::Mock => {}
        }

        tracing::info!("DNS restoration complete");
        Ok(())
    }
}

pub struct DnsGuard {
    active: bool,
    dev: String,
    strategy: DnsStrategy,
    backup_path: PathBuf,
    dry_run: bool,
}

impl DnsGuard {
    pub fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for DnsGuard {
    fn drop(&mut self) {
        if self.active && !self.dry_run {
            tracing::info!(dev = %self.dev, "DnsGuard dropped: executing automatic DNS restoration");
            if self.strategy == DnsStrategy::SystemdResolved {
                let _ = Command::new("resolvectl")
                    .args(["revert", &self.dev])
                    .output();
            } else if self.backup_path.exists() {
                let _ = fs::copy(&self.backup_path, "/etc/resolv.conf");
                let _ = fs::remove_file(&self.backup_path);
            }
        }
    }
}

fn detect_dns_strategy() -> DnsStrategy {
    #[cfg(target_os = "linux")]
    {
        // 1. Check if resolvectl or systemd-resolved is running
        if let Ok(output) = Command::new("resolvectl").arg("status").output() {
            if output.status.success() {
                return DnsStrategy::SystemdResolved;
            }
        }

        // 2. Check if NetworkManager is managing resolv.conf
        if let Ok(resolv_content) = fs::read_to_string("/etc/resolv.conf") {
            if resolv_content.contains("NetworkManager") {
                return DnsStrategy::NetworkManager;
            }
        }

        DnsStrategy::DirectResolvConf
    }

    #[cfg(not(target_os = "linux"))]
    {
        DnsStrategy::Mock
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_manager_dry_run() {
        let mut mgr = DnsManager::new("1.1.1.1", "8.8.8.8", true);
        let mut journal = UndoJournal::new();
        let guard = mgr.apply_dns("ghost0", &mut journal).unwrap();
        assert!(!journal.actions.is_empty());
        drop(guard);
        mgr.restore_dns("ghost0").unwrap();
    }
}
