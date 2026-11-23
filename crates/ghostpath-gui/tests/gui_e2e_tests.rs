use ghostpath_gui::deeplink::parse_vpn_deep_link;
use ghostpath_gui::ipc::client::IpcClient;
use ghostpath_gui::ipc::commands::{handle_check_daemon_status, handle_rpc_invoke, AppState};
use ghostpath_gui::ipc::daemon_status::DaemonStateKind;
use ghostpath_gui::ipc::mock::MockDaemon;
use serde_json::json;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_e2e_mock_backend_full_lifecycle() {
    let state = AppState {
        ipc_client: IpcClient::new("/nonexistent.sock"),
        mock_daemon: Arc::new(MockDaemon::new()),
        force_mock: true,
    };

    // 1. Initial State: Disconnected
    let status = handle_rpc_invoke(&state, "getStatus", json!({}))
        .await
        .expect("getStatus failed");
    assert_eq!(status["status"], "disconnected");
    assert_eq!(status["interface"], "ghost0");
    assert_eq!(status["traffic"]["downloadMbps"], 0.0);

    // 2. Query available nodes
    let nodes = handle_rpc_invoke(&state, "listNodes", json!({}))
        .await
        .expect("listNodes failed");
    let initial_count = nodes.as_array().expect("nodes must be an array").len();
    assert!(initial_count >= 2);

    // 3. Connect to a node
    let connect_res = handle_rpc_invoke(&state, "connect", json!({"nodeId": "mock-node-1"}))
        .await
        .expect("connect failed");
    assert_eq!(connect_res["status"], "connected");
    assert_eq!(connect_res["nodeId"], "mock-node-1");

    // 4. Verify Connected status & traffic metrics
    let active_status = handle_rpc_invoke(&state, "getStatus", json!({}))
        .await
        .expect("getStatus failed");
    assert_eq!(active_status["status"], "connected");
    assert!(active_status["traffic"]["downloadMbps"].as_f64().unwrap() > 0.0);
    assert_eq!(active_status["activeNode"]["id"], "mock-node-1");

    // 5. Deep link import simulation: parse URI -> importProfile
    let uri = "vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@195.181.162.88:443?security=reality#E2E-Frankfurt";
    let link_info = parse_vpn_deep_link(uri).expect("must parse deep link");
    assert_eq!(link_info.protocol, "VLESS");
    assert_eq!(link_info.name.as_deref(), Some("E2E-Frankfurt"));

    let import_res = handle_rpc_invoke(
        &state,
        "importProfile",
        json!({
            "raw": link_info.raw,
            "name": link_info.name.unwrap_or_default(),
        }),
    )
    .await
    .expect("importProfile failed");
    assert_eq!(import_res["type"], "single_node");
    assert_eq!(import_res["node"]["name"], "E2E-Frankfurt");

    // Verify node was added to the list
    let updated_nodes = handle_rpc_invoke(&state, "listNodes", json!({}))
        .await
        .expect("listNodes failed");
    assert_eq!(updated_nodes.as_array().unwrap().len(), initial_count + 1);

    // 6. Run Doctor Diagnostics
    let doctor = handle_rpc_invoke(&state, "runDoctor", json!({}))
        .await
        .expect("runDoctor failed");
    assert_eq!(doctor["overall_healthy"], true);
    assert_eq!(doctor["checks"].as_array().unwrap().len(), 5);

    // 7. Update and retrieve settings
    let update_settings = handle_rpc_invoke(
        &state,
        "updateSettings",
        json!({
            "killSwitch": false,
            "dnsPrimary": "9.9.9.9"
        }),
    )
    .await
    .expect("updateSettings failed");
    assert_eq!(update_settings["killSwitch"], false);
    assert_eq!(update_settings["dnsPrimary"], "9.9.9.9");

    let fetched_settings = handle_rpc_invoke(&state, "getSettings", json!({}))
        .await
        .expect("getSettings failed");
    assert_eq!(fetched_settings["killSwitch"], false);
    assert_eq!(fetched_settings["dnsPrimary"], "9.9.9.9");

    // 8. Disconnect
    let disc_res = handle_rpc_invoke(&state, "disconnect", json!({}))
        .await
        .expect("disconnect failed");
    assert_eq!(disc_res["status"], "disconnected");

    let final_status = handle_rpc_invoke(&state, "getStatus", json!({}))
        .await
        .expect("getStatus failed");
    assert_eq!(final_status["status"], "disconnected");
    assert_eq!(final_status["traffic"]["downloadMbps"], 0.0);
}

#[tokio::test]
async fn test_e2e_daemon_not_running_state_handling() {
    let temp = tempdir().unwrap();
    let sock = temp.path().join("ghostpathd.sock");

    let state = AppState {
        ipc_client: IpcClient::new(&sock),
        mock_daemon: Arc::new(MockDaemon::new()),
        force_mock: false,
    };

    // When socket does not exist and daemon is not running:
    let health = handle_check_daemon_status(&state).await;
    // Health reports either NotRunning or NotInstalled depending on whether /usr/bin/ghostpathd exists
    assert!(
        health.state == DaemonStateKind::NotRunning
            || health.state == DaemonStateKind::NotInstalled,
        "Expected NotRunning or NotInstalled, got: {:?}",
        health.state
    );
    assert!(health.fix_command.is_some());
    assert!(!health.message.is_empty());
}
