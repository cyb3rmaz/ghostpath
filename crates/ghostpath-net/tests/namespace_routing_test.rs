use std::process::Command;

use ghostpath_net::{
    RouteManager, GHOST_FWMARK, GHOST_INTERFACE_NAME, GHOST_ROUTING_TABLE, GHOST_RULE_PRIO_BYPASS,
    GHOST_RULE_PRIO_CAPTURE,
};

#[test]
fn test_namespace_policy_routing_and_recovery() {
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
            "Skipping privileged namespace test (requires root + GHOSTPATH_NETNS_ISOLATED=1 or GHOSTPATH_ALLOW_HOST_TESTS=1)"
        );
        return;
    }

    let probe = Command::new("ip")
        .args(["link", "add", "gp_probe0", "type", "dummy"])
        .output();
    match probe {
        Ok(out) if out.status.success() => {
            let _ = Command::new("ip")
                .args(["link", "del", "gp_probe0"])
                .output();
        }
        _ => {
            println!("Skipping privileged namespace test (CAP_NET_ADMIN or iproute2 unavailable)");
            return;
        }
    }

    println!("Running policy routing test in network namespace...");

    // 1. Create a dummy test interface for ghost0 so route commands succeed
    let _ = Command::new("ip")
        .args(["link", "add", GHOST_INTERFACE_NAME, "type", "dummy"])
        .output();
    let _ = Command::new("ip")
        .args(["link", "set", "dev", GHOST_INTERFACE_NAME, "up"])
        .output();

    let mut mgr = RouteManager::new(false);
    let guard = mgr
        .apply_routing(GHOST_INTERFACE_NAME, true)
        .expect("Failed to apply routing");

    // 2. Verify rules exist
    let rule_output = Command::new("ip")
        .args(["rule", "show"])
        .output()
        .expect("Failed to run ip rule show");
    let rules_str = String::from_utf8_lossy(&rule_output.stdout);

    let bypass_prio = format!("{}:", GHOST_RULE_PRIO_BYPASS);
    let capture_prio = format!("{}:", GHOST_RULE_PRIO_CAPTURE);
    let mark_hex = format!("0x{:x}", GHOST_FWMARK);

    assert!(
        rules_str.contains(&bypass_prio),
        "Bypass rule priority 5180 missing from kernel"
    );
    assert!(
        rules_str.contains(&capture_prio),
        "Capture rule priority 5181 missing from kernel"
    );
    assert!(
        rules_str.contains(&mark_hex),
        "Fwmark 0x4750 missing from kernel rules"
    );

    // 3. Verify route in table 5180
    let route_output = Command::new("ip")
        .args(["route", "show", "table", &GHOST_ROUTING_TABLE.to_string()])
        .output()
        .expect("Failed to run ip route show table 5180");
    let routes_str = String::from_utf8_lossy(&route_output.stdout);
    assert!(
        routes_str.contains(GHOST_INTERFACE_NAME),
        "Default route missing from table 5180"
    );

    // 4. Test recovery / cleanup
    drop(guard);
    mgr.cleanup_routing().expect("Failed to clean up routing");

    // Verify rules are gone
    let rule_check = Command::new("ip")
        .args(["rule", "show"])
        .output()
        .expect("Failed to run ip rule show");
    let rules_check_str = String::from_utf8_lossy(&rule_check.stdout);
    assert!(!rules_check_str.contains(&bypass_prio));
    assert!(!rules_check_str.contains(&capture_prio));

    // Clean up dummy interface
    let _ = Command::new("ip")
        .args(["link", "del", GHOST_INTERFACE_NAME])
        .output();
}
