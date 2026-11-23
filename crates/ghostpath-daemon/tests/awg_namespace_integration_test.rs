use ghostpath_daemon::DaemonEngine;
use ghostpath_proto::amnezia::{
    encode_vpn_uri, AmneziaContainer, AmneziaContainerType, AmneziaServerConfig,
};

fn generate_synthetic_amnezia_vpn_uri() -> String {
    let raw_awg_conf = r#"[Interface]
Address = 10.8.0.2/24, fd00:5180::2/64
PrivateKey = YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=
DNS = 1.1.1.1
Jc = 4
Jmin = 40
Jmax = 70
S1 = 15
S2 = 50
H1 = 1234567891
H2 = 2234567892
H3 = 3234567893
H4 = 4234567894

[Peer]
PublicKey = YmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmI=
Endpoint = 203.0.113.10:39001
AllowedIPs = 0.0.0.0/0, ::/0
PersistentKeepalive = 25
"#;

    let profile = ghostpath_proto::amnezia::parse_awg_conf(raw_awg_conf, "Synthetic AWG").unwrap();

    let server = AmneziaServerConfig {
        server_name: "Integration Test Server".to_string(),
        host_name: "203.0.113.10".to_string(),
        default_container: Some("amnezia-awg".to_string()),
        dns: vec!["1.1.1.1".to_string()],
        containers: vec![
            AmneziaContainer {
                container_name: "amnezia-awg".to_string(),
                container_type: AmneziaContainerType::Awg,
                is_supported: true,
                unsupported_reason: None,
                awg_profile: Some(profile),
                raw_json: serde_json::json!({
                    "port": "39001",
                    "transport_proto": "udp"
                }),
            },
            AmneziaContainer {
                container_name: "amnezia-openvpn-cloak".to_string(),
                container_type: AmneziaContainerType::OpenVpnCloak,
                is_supported: false,
                unsupported_reason: Some(
                    "OpenVPN over Cloak is not supported in GhostPath.".to_string(),
                ),
                awg_profile: None,
                raw_json: serde_json::json!({ "port": "443" }),
            },
        ],
    };

    encode_vpn_uri(&server).expect("Failed to encode vpn uri")
}

#[tokio::test]
async fn test_import_and_lifecycle_amnezia_vpn_uri() {
    let engine = DaemonEngine::new(true); // dry-run mode for test safety
    let vpn_uri = generate_synthetic_amnezia_vpn_uri();

    let import_res = engine
        .import_profile(&vpn_uri, Some("Custom Server".to_string()))
        .await
        .expect("Import failed");

    assert_eq!(import_res["type"], "amnezia_multi");
    assert_eq!(import_res["importedCount"], 1);

    let unsupported = import_res["unsupportedContainers"]
        .as_array()
        .expect("unsupportedContainers array");
    assert_eq!(unsupported.len(), 1);
    assert_eq!(unsupported[0]["container"], "amnezia-openvpn-cloak");

    // List nodes and verify AWG node is present
    let nodes_val = engine.list_nodes().await.expect("Failed to list nodes");
    let nodes = nodes_val.as_array().expect("Nodes array");
    assert!(!nodes.is_empty());

    let awg_node = nodes
        .iter()
        .find(|n| n["protocol"] == "AmneziaWG")
        .expect("AmneziaWG node not found");
    let node_id = awg_node["id"].as_str().unwrap().to_string();

    // Connect to the AmneziaWG node
    let connect_res = engine
        .connect(Some(node_id))
        .await
        .expect("Failed to connect");
    assert_eq!(connect_res["status"], "connected");
    assert_eq!(connect_res["interface"], "awg-ghost0");

    // Verify status
    let status_res = engine.get_status().await.expect("Failed to get status");
    assert_eq!(status_res["status"], "connected");
    assert_eq!(status_res["activeNode"]["protocol"], "AmneziaWG");

    // Disconnect
    let disconnect_res = engine.disconnect().await.expect("Failed to disconnect");
    assert_eq!(disconnect_res["status"], "disconnected");

    let status_after = engine.get_status().await.expect("Failed to get status");
    assert_eq!(status_after["status"], "disconnected");
}

#[tokio::test]
async fn test_import_and_lifecycle_amnezia_conf_file() {
    let engine = DaemonEngine::new(true);

    let raw_conf = r#"[Interface]
Address = 10.8.0.5/24
PrivateKey = YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=
DNS = 1.0.0.1
Jc = 8
Jmin = 50
Jmax = 90
S1 = 20
S2 = 40
H1 = 1111111111
H2 = 2222222222
H3 = 3333333333
H4 = 4141414141

[Peer]
PublicKey = YmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmI=
Endpoint = 198.51.100.20:41194
AllowedIPs = 0.0.0.0/0
"#;

    let import_res = engine
        .import_profile(raw_conf, Some("My Dedicated AWG".to_string()))
        .await
        .expect("Import conf failed");

    assert_eq!(import_res["type"], "single_node");
    assert_eq!(import_res["node"]["name"], "My Dedicated AWG");
    assert_eq!(import_res["node"]["protocol"], "AmneziaWG");
    assert_eq!(import_res["node"]["host"], "198.51.100.20");
    assert_eq!(import_res["node"]["port"], 41194);

    let node_id = import_res["node"]["id"].as_str().unwrap().to_string();

    let connect_res = engine
        .connect(Some(node_id))
        .await
        .expect("Connect failed");
    assert_eq!(connect_res["status"], "connected");
    assert_eq!(connect_res["interface"], "awg-ghost0");

    let disconnect_res = engine.disconnect().await.expect("Disconnect failed");
    assert_eq!(disconnect_res["status"], "disconnected");
}
