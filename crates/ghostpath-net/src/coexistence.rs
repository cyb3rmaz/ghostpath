use std::process::Command;

use ghostpath_core::error::{CoreError, Result};

use crate::journal::{JournalAction, UndoJournal};

pub const DEFAULT_EXCLUDED_CIDRS: &[&str] = &[
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "127.0.0.0/8",
    "169.254.0.0/16",
];

pub const PRIO_EXCLUDE_START: u32 = 5170;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChainMode {
    #[default]
    OtherOverOurs, // Other VPN (WireGuard, OpenVPN) connects OVER GhostPath
    OursOverOther, // GhostPath connects OVER other VPN (bind to specific device)
}

#[derive(Debug, Clone)]
pub struct CoexistenceConfig {
    pub chain_mode: ChainMode,
    pub excluded_interfaces: Vec<String>,
    pub excluded_cidrs: Vec<String>,
    pub bind_device: Option<String>,
}

impl Default for CoexistenceConfig {
    fn default() -> Self {
        Self {
            chain_mode: ChainMode::OtherOverOurs,
            excluded_interfaces: vec![
                "docker0".to_string(),
                "br-*".to_string(),
                "virbr*".to_string(),
                "tailscale0".to_string(),
            ],
            excluded_cidrs: DEFAULT_EXCLUDED_CIDRS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            bind_device: None,
        }
    }
}

pub struct CoexistenceManager {
    config: CoexistenceConfig,
    dry_run: bool,
}

impl CoexistenceManager {
    pub fn new(config: CoexistenceConfig, dry_run: bool) -> Self {
        Self { config, dry_run }
    }

    pub fn matches_excluded_interface(&self, iface_name: &str) -> bool {
        for pattern in &self.config.excluded_interfaces {
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                if iface_name.starts_with(prefix) {
                    return true;
                }
            } else if pattern == iface_name {
                return true;
            }
        }
        false
    }

    /// Applies policy bypass rules for local LAN CIDRs and excluded interfaces
    pub fn apply_coexistence_rules(&self, journal: &mut UndoJournal) -> Result<CoexistenceGuard> {
        tracing::info!(
            chain_mode = ?self.config.chain_mode,
            excluded_ifaces = ?self.config.excluded_interfaces,
            "Applying multi-VPN coexistence and LAN bypass rules"
        );

        let mut current_prio = PRIO_EXCLUDE_START;

        // 1. Install bypass rules for excluded CIDRs (e.g. 10.0.0.0/8, 192.168.0.0/16)
        for cidr in &self.config.excluded_cidrs {
            self.add_cidr_bypass_rule(current_prio, cidr, journal)?;
            current_prio += 1;
        }

        // 2. Install bypass rules for known static excluded interfaces (e.g. docker0)
        for iface in &self.config.excluded_interfaces {
            if !iface.contains('*') {
                self.add_interface_bypass_rule(current_prio, iface, journal)?;
                current_prio += 1;
            }
        }

        Ok(CoexistenceGuard {
            active: true,
            prio_start: PRIO_EXCLUDE_START,
            prio_end: current_prio,
            dry_run: self.dry_run,
        })
    }

    fn add_cidr_bypass_rule(&self, prio: u32, cidr: &str, journal: &mut UndoJournal) -> Result<()> {
        let action = JournalAction::AddedIpRule {
            priority: prio,
            fwmark: None,
            table: 254, // main
            description: format!("LAN bypass rule to {cidr}"),
        };

        if self.dry_run {
            tracing::info!(
                prio = prio,
                cidr = cidr,
                "[dry-run] ip rule add to CIDR table main"
            );
            journal.record(action);
            return Ok(());
        }

        let prio_str = prio.to_string();
        let _ = Command::new("ip")
            .args(["rule", "del", "priority", &prio_str])
            .output();

        let output = Command::new("ip")
            .args([
                "rule", "add", "to", cidr, "table", "main", "priority", &prio_str,
            ])
            .output()
            .map_err(CoreError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(cidr = cidr, error = %stderr, "Failed to add CIDR bypass rule");
        } else {
            journal.record(action);
        }

        Ok(())
    }

    fn add_interface_bypass_rule(
        &self,
        prio: u32,
        iface: &str,
        journal: &mut UndoJournal,
    ) -> Result<()> {
        let action = JournalAction::AddedIpRule {
            priority: prio,
            fwmark: None,
            table: 254, // main
            description: format!("Interface bypass rule iif {iface}"),
        };

        if self.dry_run {
            tracing::info!(
                prio = prio,
                iface = iface,
                "[dry-run] ip rule add iif table main"
            );
            journal.record(action);
            return Ok(());
        }

        let prio_str = prio.to_string();
        let _ = Command::new("ip")
            .args(["rule", "del", "priority", &prio_str])
            .output();

        let output = Command::new("ip")
            .args([
                "rule", "add", "iif", iface, "table", "main", "priority", &prio_str,
            ])
            .output()
            .map_err(CoreError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::debug!(iface = iface, error = %stderr, "Interface bypass rule skipped (interface may not exist yet)");
        } else {
            journal.record(action);
        }

        Ok(())
    }

    pub fn cleanup_coexistence_rules(&self, prio_start: u32, prio_end: u32) {
        if self.dry_run {
            return;
        }
        for p in prio_start..prio_end {
            let prio_str = p.to_string();
            let _ = Command::new("ip")
                .args(["rule", "del", "priority", &prio_str])
                .output();
        }
    }
}

pub struct CoexistenceGuard {
    active: bool,
    prio_start: u32,
    prio_end: u32,
    dry_run: bool,
}

impl CoexistenceGuard {
    pub fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for CoexistenceGuard {
    fn drop(&mut self) {
        if self.active && !self.dry_run {
            for p in self.prio_start..self.prio_end {
                let prio_str = p.to_string();
                let _ = Command::new("ip")
                    .args(["rule", "del", "priority", &prio_str])
                    .output();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_excluded_interface_pattern_matching() {
        let mgr = CoexistenceManager::new(CoexistenceConfig::default(), true);
        assert!(mgr.matches_excluded_interface("docker0"));
        assert!(mgr.matches_excluded_interface("br-1234"));
        assert!(mgr.matches_excluded_interface("virbr0"));
        assert!(mgr.matches_excluded_interface("tailscale0"));
        assert!(!mgr.matches_excluded_interface("ghost0"));
        assert!(!mgr.matches_excluded_interface("eth0"));
    }

    #[test]
    fn test_coexistence_rules_dry_run() {
        let mgr = CoexistenceManager::new(CoexistenceConfig::default(), true);
        let mut journal = UndoJournal::new();
        let guard = mgr.apply_coexistence_rules(&mut journal).unwrap();
        assert!(journal.actions.len() >= 5);
        drop(guard);
    }
}
