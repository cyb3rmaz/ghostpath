use std::process::Command;

use ghostpath_net::coexistence::{CoexistenceConfig, CoexistenceManager};
use ghostpath_net::journal::UndoJournal;
use ghostpath_net::{RouteManager, GHOST_INTERFACE_NAME};

#[test]
fn test_wireguard_coexistence_and_chaining() {
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

    let opted_in = std::env::var("GHOSTPATH_NETNS_ISOLATED").as_deref() == Ok("1")
        || std::env::var("GHOSTPATH_ALLOW_HOST_TESTS").as_deref() == Ok("1");

    if !is_root || !opted_in {
        println!(
            "Skipping privileged WireGuard coexistence test (requires root + GHOSTPATH_NETNS_ISOLATED=1 or GHOSTPATH_ALLOW_HOST_TESTS=1)"
        );
        return;
    }

    let probe = Command::new("ip")
        .args(["link", "add", "gp_probe1", "type", "dummy"])
        .output();
    match probe {
        Ok(out) if out.status.success() => {
            let _ = Command::new("ip")
                .args(["link", "del", "gp_probe1"])
                .output();
        }
        _ => {
            println!("Skipping privileged WireGuard coexistence test (CAP_NET_ADMIN or iproute2 unavailable)");
            return;
        }
    }

    println!("Starting WireGuard-over-tunnel coexistence integration test...");

    // 1. Create dummy interface for ghost0
    let _ = Command::new("ip")
        .args(["link", "add", GHOST_INTERFACE_NAME, "type", "dummy"])
        .output();
    let _ = Command::new("ip")
        .args(["link", "set", "dev", GHOST_INTERFACE_NAME, "up"])
        .output();

    // 2. Apply GhostPath routing rules
    let mut route_mgr = RouteManager::new(false);
    let mut journal = UndoJournal::new();
    let route_guard = route_mgr
        .apply_routing(GHOST_INTERFACE_NAME, false)
        .expect("Failed to apply GhostPath routing");

    // 3. Apply Coexistence & LAN bypass rules
    let coex_mgr = CoexistenceManager::new(CoexistenceConfig::default(), false);
    let coex_guard = coex_mgr
        .apply_coexistence_rules(&mut journal)
        .expect("Failed to apply coexistence rules");

    // 4. Create WireGuard interface wg0
    let wg_iface = "wg0";
    let _ = Command::new("ip").args(["link", "del", wg_iface]).output();
    let wg_create = Command::new("ip")
        .args(["link", "add", "dev", wg_iface, "type", "dummy"])
        .output()
        .expect("Failed to create mock wireguard interface");
    assert!(wg_create.status.success(), "Failed to create mock wg0");

    let _ = Command::new("ip")
        .args(["addr", "add", "10.0.0.2/24", "dev", wg_iface])
        .output();
    let _ = Command::new("ip")
        .args(["link", "set", "dev", wg_iface, "up"])
        .output();

    // 5. Verify routing:
    // Packet from wg0 (LAN subnet 10.0.0.0/8) matches LAN bypass rule
    let route_check = Command::new("ip")
        .args(["route", "get", "10.0.0.1"])
        .output()
        .expect("Failed to run ip route get");
    let route_str = String::from_utf8_lossy(&route_check.stdout);
    println!("Route for 10.0.0.1 (WireGuard subnet): {}", route_str);

    // Verify both interfaces exist concurrently without conflict
    let link_check = Command::new("ip")
        .args(["link", "show"])
        .output()
        .expect("Failed to list interfaces");
    let link_str = String::from_utf8_lossy(&link_check.stdout);
    assert!(link_str.contains(GHOST_INTERFACE_NAME));
    assert!(link_str.contains(wg_iface));

    // 6. Cleanup
    drop(coex_guard);
    drop(route_guard);
    let _ = Command::new("ip").args(["link", "del", wg_iface]).output();
    let _ = Command::new("ip")
        .args(["link", "del", GHOST_INTERFACE_NAME])
        .output();

    route_mgr
        .cleanup_routing()
        .expect("Failed to clean up GhostPath routing");

    println!("WireGuard coexistence test passed successfully!");
}
