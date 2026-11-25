use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub id: String,
    pub name: String,
    pub passed: bool,
    pub severity: String, // "critical", "warning", "info"
    pub details: String,
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub overall_healthy: bool,
}

pub struct SystemDoctor;

impl SystemDoctor {
    pub fn run_diagnostics() -> DoctorReport {
        let checks = vec![
            Self::check_tun_device(),
            Self::check_privileges(),
            Self::check_rp_filter(),
            Self::check_ip_forward(),
            Self::check_dns_resolver(),
            Self::check_docker(),
            Self::check_ipv6(),
            Self::check_amneziawg_engine(),
            Self::check_geo_timezone_support(),
            Self::check_conflicting_vpn_interfaces(),
        ];

        let overall_healthy = checks.iter().all(|c| c.passed || c.severity != "critical");

        DoctorReport {
            checks,
            overall_healthy,
        }
    }

    fn check_tun_device() -> DoctorCheck {
        let path = Path::new("/dev/net/tun");
        let exists = path.exists();
        DoctorCheck {
            id: "tun".to_string(),
            name: "Kernel TUN Device (/dev/net/tun)".to_string(),
            passed: exists,
            severity: "critical".to_string(),
            details: if exists {
                "TUN driver is loaded and available at /dev/net/tun".to_string()
            } else {
                "TUN driver (/dev/net/tun) is missing. Kernel cannot allocate virtual interfaces."
                    .to_string()
            },
            fix: if !exists {
                Some("sudo modprobe tun && sudo mkdir -p /dev/net && sudo mknod /dev/net/tun c 10 200".to_string())
            } else {
                None
            },
        }
    }

    fn check_privileges() -> DoctorCheck {
        let is_root = {
            #[cfg(unix)]
            {
                nix::unistd::Uid::current().is_root()
            }
            #[cfg(not(unix))]
            {
                false
            }
        };

        // Check file capabilities if available
        let has_caps = Self::check_effective_caps();
        let passed = is_root || has_caps;

        DoctorCheck {
            id: "privs".to_string(),
            name: "Daemon Capabilities & Privileges".to_string(),
            passed,
            severity: "critical".to_string(),
            details: if is_root {
                "Running with UID 0 (root)".to_string()
            } else if has_caps {
                "Running with network capabilities (CAP_NET_ADMIN)".to_string()
            } else {
                "Insufficient capabilities to manipulate routing tables and TUN devices".to_string()
            },
            fix: if !passed {
                Some("sudo setcap cap_net_admin,cap_net_raw,cap_net_bind_service+ep /usr/bin/ghostpathd".to_string())
            } else {
                None
            },
        }
    }

    fn check_rp_filter() -> DoctorCheck {
        let rp_filter_all = fs::read_to_string("/proc/sys/net/ipv4/conf/all/rp_filter")
            .unwrap_or_default()
            .trim()
            .parse::<u8>()
            .unwrap_or(0);

        let is_strict = rp_filter_all == 1;

        DoctorCheck {
            id: "rp_filter".to_string(),
            name: "Reverse Path Filtering (rp_filter)".to_string(),
            passed: !is_strict,
            severity: "warning".to_string(),
            details: if is_strict {
                "Strict rp_filter (1) detected. Asymmetric tunnel routing may cause packet drops."
                    .to_string()
            } else {
                format!(
                    "rp_filter is set to loose ({}) or disabled, compatible with policy routing",
                    rp_filter_all
                )
            },
            fix: if is_strict {
                Some("sudo sysctl -w net.ipv4.conf.all.rp_filter=2 net.ipv4.conf.default.rp_filter=2".to_string())
            } else {
                None
            },
        }
    }

    fn check_ip_forward() -> DoctorCheck {
        let ip_forward = fs::read_to_string("/proc/sys/net/ipv4/ip_forward")
            .unwrap_or_default()
            .trim()
            .parse::<u8>()
            .unwrap_or(0);

        let enabled = ip_forward == 1;

        DoctorCheck {
            id: "ip_forward".to_string(),
            name: "IPv4 Packet Forwarding (net.ipv4.ip_forward)".to_string(),
            passed: true, // Not critical for single-node client, but recommended for chaining
            severity: "info".to_string(),
            details: if enabled {
                "Packet forwarding is active (ideal for WireGuard-over-GhostPath chaining)"
                    .to_string()
            } else {
                "Packet forwarding is disabled (standard host mode)".to_string()
            },
            fix: if !enabled {
                Some("sudo sysctl -w net.ipv4.ip_forward=1".to_string())
            } else {
                None
            },
        }
    }

    fn check_dns_resolver() -> DoctorCheck {
        let has_resolved = Path::new("/run/systemd/resolve/resolv.conf").exists()
            || Path::new("/run/systemd/resolve/stub-resolv.conf").exists();

        let has_etc_resolv = Path::new("/etc/resolv.conf").exists();

        DoctorCheck {
            id: "dns".to_string(),
            name: "DNS Resolver Subsystem".to_string(),
            passed: has_resolved || has_etc_resolv,
            severity: "critical".to_string(),
            details: if has_resolved {
                "systemd-resolved active; per-link domain routing (~.) will prevent DNS leaks"
                    .to_string()
            } else if has_etc_resolv {
                "Direct /etc/resolv.conf detected; fallback atomic backup/restore will be used"
                    .to_string()
            } else {
                "No active DNS configuration detected".to_string()
            },
            fix: if !has_resolved {
                Some("sudo systemctl enable --now systemd-resolved".to_string())
            } else {
                None
            },
        }
    }

    fn check_docker() -> DoctorCheck {
        let docker_bridge = Path::new("/sys/class/net/docker0").exists();

        DoctorCheck {
            id: "docker".to_string(),
            name: "Docker Bridge Interference (docker0)".to_string(),
            passed: true, // GhostPath coexistence rules handle this automatically
            severity: "info".to_string(),
            details: if docker_bridge {
                "docker0 bridge detected; GhostPath coexistence rules isolate container traffic"
                    .to_string()
            } else {
                "No Docker bridge interference detected".to_string()
            },
            fix: None,
        }
    }

    fn check_ipv6() -> DoctorCheck {
        let disabled = fs::read_to_string("/proc/sys/net/ipv6/conf/all/disable_ipv6")
            .unwrap_or_default()
            .trim()
            .parse::<u8>()
            .unwrap_or(0)
            == 1;

        DoctorCheck {
            id: "ipv6".to_string(),
            name: "IPv6 Leak Protection Posture".to_string(),
            passed: true,
            severity: "info".to_string(),
            details: if disabled {
                "IPv6 is disabled at kernel level (no leak vector)".to_string()
            } else {
                "IPv6 enabled; GhostPath table 5180 installs default blackhole route to prevent leaks".to_string()
            },
            fix: None,
        }
    }

    fn check_conflicting_vpn_interfaces() -> DoctorCheck {
        let mut conflicts = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if (name.starts_with("wg")
                    || name.starts_with("awg")
                    || name.starts_with("tailscale")
                    || name.starts_with("tun")
                    || name.starts_with("tap"))
                    && name != "ghost0"
                    && name != "awg-ghost0"
                {
                    conflicts.push(name);
                }
            }
        }

        let has_conflicts = !conflicts.is_empty();

        DoctorCheck {
            id: "vpn_coexistence".to_string(),
            name: "Concurrent VPN Interfaces".to_string(),
            passed: true, // GhostPath policy routing supports coexistence
            severity: "info".to_string(),
            details: if has_conflicts {
                format!(
                    "Active foreign VPN interfaces found: {}. GhostPath routing table 5180 maintains coexistence.",
                    conflicts.join(", ")
                )
            } else {
                "No conflicting VPN tunnels detected".to_string()
            },
            fix: None,
        }
    }

    fn check_amneziawg_engine() -> DoctorCheck {
        let has_kernel_mod = Path::new("/sys/module/amneziawg").exists();
        let binary_path = crate::tunnel::AwgBackend::find_binary();

        let passed = has_kernel_mod || binary_path.is_some();

        DoctorCheck {
            id: "amneziawg_engine".to_string(),
            name: "AmneziaWG Userspace & Kernel Runtime".to_string(),
            passed,
            severity: "warning".to_string(),
            details: if has_kernel_mod {
                "AmneziaWG out-of-tree kernel module (amneziawg.ko) is loaded".to_string()
            } else if let Some(bin) = binary_path {
                format!("AmneziaWG userspace engine found at {}", bin.display())
            } else {
                "Neither amneziawg-go binary nor amneziawg kernel module was detected. AmneziaWG profiles will be unavailable until installed.".to_string()
            },
            fix: if !passed {
                Some("Install ghostpath-vpn package with bundled amneziawg-go, or place amneziawg-go into /usr/bin/amneziawg-go".to_string())
            } else {
                None
            },
        }
    }

    fn check_geo_timezone_support() -> DoctorCheck {
        let mgr = ghostpath_net::GeoSpoofManager::new(true);
        let (has_timedatectl, has_geoclue) = mgr.detect_capabilities();
        let active_tz = mgr.read_active_timezone();
        let zoneinfo_ok = Path::new("/usr/share/zoneinfo").exists();
        let passed = zoneinfo_ok || has_timedatectl;

        DoctorCheck {
            id: "geo_timezone".to_string(),
            name: "System Timezone & GeoClue Spoofing".to_string(),
            passed,
            severity: "info".to_string(),
            details: format!(
                "Active TZ: {}; timedatectl: {}; GeoClue2: {}",
                active_tz,
                if has_timedatectl {
                    "ready"
                } else {
                    "fallback (/etc/localtime)"
                },
                if has_geoclue {
                    "ready (/etc/geolocation)"
                } else {
                    "not installed (timezone spoofing still works)"
                }
            ),
            fix: if !has_geoclue {
                Some("Optional for desktop GPS/browser geolocation spoofing: sudo apt install geoclue-2.0".to_string())
            } else {
                None
            },
        }
    }

    fn check_effective_caps() -> bool {
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("CapEff:") {
                    let cap_hex = line.trim_start_matches("CapEff:").trim();
                    if let Ok(caps) = u64::from_str_radix(cap_hex, 16) {
                        // CAP_NET_ADMIN is bit 12 (0x1000)
                        return (caps & (1 << 12)) != 0;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_doctor_run() {
        let report = SystemDoctor::run_diagnostics();
        assert!(!report.checks.is_empty());
        assert!(report.checks.iter().any(|c| c.id == "tun"));
        assert!(report.checks.iter().any(|c| c.id == "privs"));
        assert!(report.checks.iter().any(|c| c.id == "rp_filter"));
        assert!(report.checks.iter().any(|c| c.id == "dns"));
        assert!(report.checks.iter().any(|c| c.id == "docker"));
        assert!(report.checks.iter().any(|c| c.id == "ipv6"));
        assert!(report.checks.iter().any(|c| c.id == "amneziawg_engine"));
    }
}
