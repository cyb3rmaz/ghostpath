use crate::journal::{JournalAction, UndoJournal};
use ghostpath_core::error::{CoreError, Result};
use std::process::Command;
use tracing::{debug, info, warn};

pub const GHOST_NFT_TABLE: &str = "ghostpath";
pub const GHOST_NFT_FAMILY: &str = "inet";

pub struct KillSwitchGuard {
    active: bool,
    dry_run: bool,
}

impl KillSwitchGuard {
    pub fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for KillSwitchGuard {
    fn drop(&mut self) {
        if self.active && !self.dry_run {
            info!("KillSwitchGuard dropped: removing nftables table ghostpath");
            let _ = KillSwitch::disable(false);
        }
    }
}

pub struct KillSwitch;

impl KillSwitch {
    /// Enables the strict nftables kill switch in table `ghostpath` (family `inet`).
    ///
    /// When active, drops any traffic leaving non-tunnel physical interfaces
    /// unless marked with fwmark 0x4750 (the proxy core itself) or destined for
    /// local loopback / RFC1918 LAN (if allow_lan is true).
    pub fn enable(
        dry_run: bool,
        allow_lan: bool,
        journal: &mut UndoJournal,
    ) -> Result<KillSwitchGuard> {
        info!(
            dry_run = dry_run,
            allow_lan = allow_lan,
            table = GHOST_NFT_TABLE,
            "Activating nftables kill switch"
        );

        if dry_run {
            info!("[dry-run] nft add table inet ghostpath");
            info!("[dry-run] nft add chain inet ghostpath output {{ type filter hook output priority -10; }}");
            info!(
                "[dry-run] nft rules installed: lo accept, mark 0x4750 accept, ghost0 accept, drop"
            );
            journal.record(JournalAction::CreatedNftTable {
                family: GHOST_NFT_FAMILY.to_string(),
                name: GHOST_NFT_TABLE.to_string(),
            });
            return Ok(KillSwitchGuard {
                active: true,
                dry_run: true,
            });
        }

        // 1. Remove stale table if exists
        let _ = run_nft(&["delete", "table", GHOST_NFT_FAMILY, GHOST_NFT_TABLE]);

        // 2. Create inet table
        run_nft(&["add", "table", GHOST_NFT_FAMILY, GHOST_NFT_TABLE])?;

        // 3. Create output chain
        run_nft(&[
            "add",
            "chain",
            GHOST_NFT_FAMILY,
            GHOST_NFT_TABLE,
            "output",
            "{",
            "type",
            "filter",
            "hook",
            "output",
            "priority",
            "-10",
            ";",
            "policy",
            "accept",
            ";",
            "}",
        ])?;

        // 4. Accept loopback
        run_nft(&[
            "add",
            "rule",
            GHOST_NFT_FAMILY,
            GHOST_NFT_TABLE,
            "output",
            "oifname",
            "lo",
            "accept",
        ])?;

        // 5. Accept proxy core sockets with fwmark 0x4750 (18256)
        run_nft(&[
            "add",
            "rule",
            GHOST_NFT_FAMILY,
            GHOST_NFT_TABLE,
            "output",
            "mark",
            "0x4750",
            "accept",
        ])?;

        // 6. Accept traffic outbound on ghost0 tunnel
        run_nft(&[
            "add",
            "rule",
            GHOST_NFT_FAMILY,
            GHOST_NFT_TABLE,
            "output",
            "oifname",
            "ghost0",
            "accept",
        ])?;

        // 7. Accept DHCP requests so IP leasing does not break
        run_nft(&[
            "add",
            "rule",
            GHOST_NFT_FAMILY,
            GHOST_NFT_TABLE,
            "output",
            "udp",
            "dport",
            "{",
            "67,",
            "68",
            "}",
            "accept",
        ])?;

        // 8. Accept RFC1918 local LAN traffic if allow_lan is true
        if allow_lan {
            run_nft(&[
                "add",
                "rule",
                GHOST_NFT_FAMILY,
                GHOST_NFT_TABLE,
                "output",
                "ip",
                "daddr",
                "{",
                "10.0.0.0/8,",
                "172.16.0.0/12,",
                "192.168.0.0/16,",
                "224.0.0.0/4,",
                "255.255.255.255/32",
                "}",
                "accept",
            ])?;
            run_nft(&[
                "add",
                "rule",
                GHOST_NFT_FAMILY,
                GHOST_NFT_TABLE,
                "output",
                "ip6",
                "daddr",
                "{",
                "fe80::/10,",
                "ff00::/8",
                "}",
                "accept",
            ])?;
        }

        // 9. Drop all remaining output traffic
        run_nft(&[
            "add",
            "rule",
            GHOST_NFT_FAMILY,
            GHOST_NFT_TABLE,
            "output",
            "drop",
        ])?;

        journal.record(JournalAction::CreatedNftTable {
            family: GHOST_NFT_FAMILY.to_string(),
            name: GHOST_NFT_TABLE.to_string(),
        });

        info!("nftables kill switch rules active in table inet ghostpath");

        Ok(KillSwitchGuard {
            active: true,
            dry_run: false,
        })
    }

    /// Disables the kill switch by deleting table `ghostpath` (family `inet`).
    pub fn disable(dry_run: bool) -> Result<()> {
        info!(
            dry_run = dry_run,
            table = GHOST_NFT_TABLE,
            "Deactivating nftables kill switch"
        );
        if dry_run {
            return Ok(());
        }

        let _ = run_nft(&["delete", "table", GHOST_NFT_FAMILY, GHOST_NFT_TABLE]);
        Ok(())
    }
}

fn run_nft(args: &[&str]) -> Result<()> {
    debug!("Running nft {:?}", args);
    let out = Command::new("nft")
        .args(args)
        .output()
        .map_err(CoreError::Io)?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        warn!("nft error (args: {:?}): {}", args, err);
        return Err(CoreError::Internal(format!("nft command failed: {}", err)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_killswitch_dry_run() {
        let mut journal = UndoJournal::default();
        let guard = KillSwitch::enable(true, true, &mut journal).unwrap();
        assert_eq!(journal.actions.len(), 1);
        drop(guard);
        KillSwitch::disable(true).unwrap();
    }
}
