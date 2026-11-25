use std::process::Command;

use ghostpath_core::error::{CoreError, Result};

use crate::journal::{JournalAction, UndoJournal};
use crate::{GHOST_FWMARK, GHOST_ROUTING_TABLE, GHOST_RULE_PRIO_BYPASS, GHOST_RULE_PRIO_CAPTURE};

pub struct RouteManager {
    journal: UndoJournal,
    dry_run: bool,
}

impl Default for RouteManager {
    fn default() -> Self {
        Self::new(false)
    }
}

impl RouteManager {
    pub fn new(dry_run: bool) -> Self {
        Self {
            journal: UndoJournal::new(),
            dry_run,
        }
    }

    /// Recovers from any stale state left by an earlier crash or SIGKILL
    pub fn recover() -> Result<usize> {
        if let Some(journal) = UndoJournal::load()? {
            tracing::info!(
                count = journal.actions.len(),
                "Found stale undo journal; rolling back past actions..."
            );
            let mut count = 0;
            for action in journal.actions.iter().rev() {
                if let Err(e) = Self::revert_action(action) {
                    tracing::warn!(action = ?action, error = %e, "Failed to revert action during recovery (continuing)");
                } else {
                    count += 1;
                }
            }
            UndoJournal::remove_journal_file()?;
            tracing::info!(reverted = count, "Undo journal recovery complete");
            Ok(count)
        } else {
            tracing::debug!("No stale undo journal found");
            Ok(0)
        }
    }

    /// Sets up the complete loop-free policy routing for ghost0
    pub fn apply_routing(&mut self, dev: &str, ipv6_enabled: bool) -> Result<RouteGuard> {
        tracing::info!(
            dev = dev,
            table = GHOST_ROUTING_TABLE,
            fwmark = GHOST_FWMARK,
            "Applying GhostPath policy routing"
        );

        // 1. Add bypass rule (priority 5180): packets with fwmark 0x4750 lookup table main
        self.add_ip_rule(
            GHOST_RULE_PRIO_BYPASS,
            Some(GHOST_FWMARK),
            "main",
            false,
            "Bypass rule for proxy core outbound sockets",
        )?;

        // 2. Add capture rule (priority 5181): packets without fwmark 0x4750 lookup table 5180
        self.add_ip_rule(
            GHOST_RULE_PRIO_CAPTURE,
            Some(GHOST_FWMARK),
            &GHOST_ROUTING_TABLE.to_string(),
            true, // not fwmark
            "Capture rule diverting traffic to GhostPath table 5180",
        )?;

        // 3. Add default route in table 5180: default dev ghost0
        self.add_table_route(dev, "default", GHOST_ROUTING_TABLE, false)?;

        // 4. IPv6 handling: either tunnel fully or blackhole to prevent IPv6 leaks
        if ipv6_enabled {
            self.add_ip_rule_v6(
                GHOST_RULE_PRIO_BYPASS,
                Some(GHOST_FWMARK),
                "main",
                false,
                "IPv6 bypass rule for proxy core outbound sockets",
            )?;
            self.add_ip_rule_v6(
                GHOST_RULE_PRIO_CAPTURE,
                Some(GHOST_FWMARK),
                &GHOST_ROUTING_TABLE.to_string(),
                true,
                "IPv6 capture rule diverting traffic to GhostPath table 5180",
            )?;
            self.add_table_route(dev, "default", GHOST_ROUTING_TABLE, true)?;
        } else {
            self.add_ip_rule_v6(
                GHOST_RULE_PRIO_BYPASS,
                Some(GHOST_FWMARK),
                "main",
                false,
                "IPv6 bypass rule for proxy core outbound sockets",
            )?;
            self.add_ip_rule_v6(
                GHOST_RULE_PRIO_CAPTURE,
                Some(GHOST_FWMARK),
                &GHOST_ROUTING_TABLE.to_string(),
                true,
                "IPv6 blackhole capture rule preventing leaks",
            )?;
            self.add_table_blackhole_v6(GHOST_ROUTING_TABLE)?;
        }

        // Save journal to disk for crash safety
        if !self.dry_run {
            self.journal.save()?;
        }

        tracing::info!("Policy routing successfully configured");
        Ok(RouteGuard {
            active: true,
            dry_run: self.dry_run,
        })
    }

    /// Cleanly removes all GhostPath policy routing rules and routes
    pub fn cleanup_routing(&self) -> Result<()> {
        tracing::info!("Cleaning up GhostPath policy routing rules and table 5180");

        if self.dry_run {
            tracing::info!("Dry-run mode: skipping kernel mutation during cleanup");
            return Ok(());
        }

        // Remove rules (IPv4 and IPv6)
        let _ = run_cmd(
            "ip",
            &[
                "rule",
                "del",
                "priority",
                &GHOST_RULE_PRIO_BYPASS.to_string(),
            ],
        );
        let _ = run_cmd(
            "ip",
            &[
                "rule",
                "del",
                "priority",
                &GHOST_RULE_PRIO_CAPTURE.to_string(),
            ],
        );
        let _ = run_cmd(
            "ip",
            &[
                "-6",
                "rule",
                "del",
                "priority",
                &GHOST_RULE_PRIO_BYPASS.to_string(),
            ],
        );
        let _ = run_cmd(
            "ip",
            &[
                "-6",
                "rule",
                "del",
                "priority",
                &GHOST_RULE_PRIO_CAPTURE.to_string(),
            ],
        );

        // Flush dedicated table 5180 only
        let _ = run_cmd(
            "ip",
            &["route", "flush", "table", &GHOST_ROUTING_TABLE.to_string()],
        );
        let _ = run_cmd(
            "ip",
            &[
                "-6",
                "route",
                "flush",
                "table",
                &GHOST_ROUTING_TABLE.to_string(),
            ],
        );

        let _ = UndoJournal::remove_journal_file();
        tracing::info!("GhostPath routing cleanup complete");
        Ok(())
    }

    fn add_ip_rule(
        &mut self,
        priority: u32,
        fwmark: Option<u32>,
        table: &str,
        not_mark: bool,
        desc: &str,
    ) -> Result<()> {
        let action = JournalAction::AddedIpRule {
            priority,
            fwmark,
            table: table.parse().unwrap_or(254),
            description: desc.to_string(),
        };

        if self.dry_run {
            tracing::info!(
                priority = priority,
                table = table,
                not_mark = not_mark,
                desc = desc,
                "[dry-run] ip rule add"
            );
            self.journal.record(action);
            return Ok(());
        }

        // Delete any stale existing rule at this priority first for idempotency
        let _ = run_cmd("ip", &["rule", "del", "priority", &priority.to_string()]);

        let mut args = vec!["rule", "add"];
        let prio_str = priority.to_string();
        args.extend_from_slice(&["priority", &prio_str]);

        let mark_str;
        if let Some(mark) = fwmark {
            mark_str = format!("0x{:x}", mark);
            if not_mark {
                args.extend_from_slice(&["not", "fwmark", &mark_str]);
            } else {
                args.extend_from_slice(&["fwmark", &mark_str]);
            }
        }

        args.extend_from_slice(&["lookup", table]);

        let output = run_cmd("ip", &args)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::Internal(format!(
                "Failed to add ip rule priority {priority}: {stderr}"
            )));
        }

        self.journal.record(action);
        Ok(())
    }

    fn add_ip_rule_v6(
        &mut self,
        priority: u32,
        fwmark: Option<u32>,
        table: &str,
        not_mark: bool,
        desc: &str,
    ) -> Result<()> {
        if self.dry_run {
            tracing::info!(
                priority = priority,
                table = table,
                not_mark = not_mark,
                desc = desc,
                "[dry-run] ip -6 rule add"
            );
            return Ok(());
        }

        let _ = run_cmd(
            "ip",
            &["-6", "rule", "del", "priority", &priority.to_string()],
        );

        let mut args = vec!["-6", "rule", "add"];
        let prio_str = priority.to_string();
        args.extend_from_slice(&["priority", &prio_str]);

        let mark_str;
        if let Some(mark) = fwmark {
            mark_str = format!("0x{:x}", mark);
            if not_mark {
                args.extend_from_slice(&["not", "fwmark", &mark_str]);
            } else {
                args.extend_from_slice(&["fwmark", &mark_str]);
            }
        }

        args.extend_from_slice(&["lookup", table]);

        let output = run_cmd("ip", &args)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(priority = priority, error = %stderr, "Failed to add IPv6 rule (system may lack IPv6)");
        }

        Ok(())
    }

    fn add_table_route(&mut self, dev: &str, dest: &str, table: u32, ipv6: bool) -> Result<()> {
        let action = JournalAction::AddedRoute {
            destination: dest.to_string(),
            table,
            device: dev.to_string(),
        };

        if self.dry_run {
            tracing::info!(
                dev = dev,
                dest = dest,
                table = table,
                ipv6 = ipv6,
                "[dry-run] ip route add"
            );
            self.journal.record(action);
            return Ok(());
        }

        let table_str = table.to_string();
        let mut args = if ipv6 {
            vec!["-6", "route", "add", dest, "dev", dev, "table", &table_str]
        } else {
            vec!["route", "add", dest, "dev", dev, "table", &table_str]
        };

        // If route already exists in table, replace it
        let output = run_cmd("ip", &args)?;
        if !output.status.success() {
            args[if ipv6 { 2 } else { 1 }] = "replace";
            let replace_out = run_cmd("ip", &args)?;
            if !replace_out.status.success() {
                let stderr = String::from_utf8_lossy(&replace_out.stderr);
                return Err(CoreError::Internal(format!(
                    "Failed to add route {dest} in table {table}: {stderr}"
                )));
            }
        }

        self.journal.record(action);
        Ok(())
    }

    fn add_table_blackhole_v6(&mut self, table: u32) -> Result<()> {
        if self.dry_run {
            tracing::info!(table = table, "[dry-run] ip -6 route add blackhole default");
            return Ok(());
        }
        let table_str = table.to_string();
        let output = run_cmd(
            "ip",
            &[
                "-6",
                "route",
                "replace",
                "blackhole",
                "default",
                "table",
                &table_str,
            ],
        )?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(table = table, error = %stderr, "Failed to add IPv6 blackhole route (system may lack IPv6)");
        }
        Ok(())
    }

    fn revert_action(action: &JournalAction) -> Result<()> {
        match action {
            JournalAction::AddedIpRule { priority, .. } => {
                let prio_str = priority.to_string();
                let _ = run_cmd("ip", &["rule", "del", "priority", &prio_str]);
                let _ = run_cmd("ip", &["-6", "rule", "del", "priority", &prio_str]);
            }
            JournalAction::AddedRoute {
                destination,
                table,
                device,
            } => {
                let table_str = table.to_string();
                let _ = run_cmd(
                    "ip",
                    &[
                        "route",
                        "del",
                        destination,
                        "dev",
                        device,
                        "table",
                        &table_str,
                    ],
                );
            }
            JournalAction::CreatedInterface { name } => {
                let _ = run_cmd("ip", &["link", "del", name]);
            }
            JournalAction::CreatedNftTable { family, name } => {
                let _ = run_cmd("nft", &["delete", "table", family, name]);
            }
            _ => {}
        }
        Ok(())
    }
}

pub struct RouteGuard {
    active: bool,
    dry_run: bool,
}

impl RouteGuard {
    pub fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for RouteGuard {
    fn drop(&mut self) {
        if self.active && !self.dry_run {
            tracing::info!("RouteGuard dropped: executing automatic rollback of policy routing");
            let mgr = RouteManager::default();
            let _ = mgr.cleanup_routing();
        }
    }
}

fn run_cmd(prog: &str, args: &[&str]) -> Result<std::process::Output> {
    let bin = ghostpath_core::GhostPaths::resolve_trusted_system_bin(prog);
    Command::new(bin).args(args).output().map_err(CoreError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GHOST_INTERFACE_NAME;

    #[test]
    fn test_route_manager_dry_run() {
        let mut mgr = RouteManager::new(true);
        let guard = mgr.apply_routing(GHOST_INTERFACE_NAME, true).unwrap();
        assert_eq!(mgr.journal.actions.len(), 4);
        drop(guard);
        mgr.cleanup_routing().unwrap();
    }
}
