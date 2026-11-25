use ghostpath_daemon::tunnel::{AwgBackend, TunnelBackend};
use ghostpath_net::journal::{JournalAction, UndoJournal};
use ghostpath_proto::amnezia::{
    AwgInterfaceConfig, AwgObfuscationConfig, AwgPeerConfig, AwgProfileConfig,
};

fn create_sample_awg_profile() -> AwgProfileConfig {
    AwgProfileConfig {
        id: "test-awg-profile".to_string(),
        name: "Sample AWG".to_string(),
        interface: AwgInterfaceConfig {
            private_key: "YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=".to_string(),
            address: vec!["10.8.0.2/24".to_string(), "fd00:5180::2/64".to_string()],
            dns: vec!["1.1.1.1".to_string()],
            mtu: Some(1420),
            obfuscation: AwgObfuscationConfig {
                jc: Some(4),
                jmin: Some(40),
                jmax: Some(70),
                s1: Some(15),
                s2: Some(50),
                s3: Some(10),
                s4: Some(20),
                h1: Some(1234567891),
                h2: Some(2234567892),
                h3: Some(3234567893),
                h4: Some(4234567894),
                i1: Some("<b 0x01020304>".to_string()),
                i2: None,
                i3: None,
                i4: None,
                i5: None,
            },
        },
        peer: AwgPeerConfig {
            public_key: "YmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmI=".to_string(),
            preshared_key: Some("Y2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2M=".to_string()),
            endpoint: "203.0.113.10:39001".to_string(),
            allowed_ips: vec!["0.0.0.0/0".to_string(), "::/0".to_string()],
            persistent_keepalive: Some(25),
        },
    }
}

#[test]
fn test_base64_to_hex_conversion() {
    let b64 = "YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE="; // 32 'a's (0x61)
    let hex = AwgBackend::base64_to_hex(b64).expect("Conversion failed");
    assert_eq!(hex.len(), 64);
    assert_eq!(
        hex,
        "6161616161616161616161616161616161616161616161616161616161616161"
    );

    // Invalid length
    let bad_b64 = "YWFh";
    assert!(AwgBackend::base64_to_hex(bad_b64).is_err());
}

#[test]
fn test_build_uapi_config() {
    let backend = AwgBackend::new(Some("awg-test0"), Some(0x4750));
    let profile = create_sample_awg_profile();

    let uapi = backend
        .build_uapi_config(&profile)
        .expect("Failed to build UAPI config");

    assert!(uapi.contains("set=1\n"));
    assert!(uapi.contains(
        "private_key=6161616161616161616161616161616161616161616161616161616161616161\n"
    ));
    assert!(uapi.contains("jc=4\n"));
    assert!(uapi.contains("jmin=40\n"));
    assert!(uapi.contains("jmax=70\n"));
    assert!(uapi.contains("s1=15\n"));
    assert!(uapi.contains("s2=50\n"));
    assert!(uapi.contains("s3=10\n"));
    assert!(uapi.contains("s4=20\n"));
    assert!(uapi.contains("h1=1234567891\n"));
    assert!(uapi.contains("h2=2234567892\n"));
    assert!(uapi.contains("h3=3234567893\n"));
    assert!(uapi.contains("h4=4234567894\n"));
    assert!(uapi.contains("i1=<b 0x01020304>\n"));
    assert!(uapi
        .contains("public_key=6262626262626262626262626262626262626262626262626262626262626262\n"));
    assert!(uapi.contains(
        "preshared_key=6363636363636363636363636363636363636363636363636363636363636363\n"
    ));
    assert!(uapi.contains("endpoint=203.0.113.10:39001\n"));
    assert!(uapi.contains("allowed_ip=0.0.0.0/0\n"));
    assert!(uapi.contains("allowed_ip=::/0\n"));
    assert!(uapi.contains("persistent_keepalive_interval=25\n"));
}

#[tokio::test]
async fn test_awg_backend_dry_run_lifecycle() {
    let mut backend = AwgBackend::new(Some("awg-dry0"), Some(0x4750));
    let profile = create_sample_awg_profile();
    let mut journal = UndoJournal::new();

    assert!(!backend.is_running().await);

    // Start in dry_run mode
    backend
        .start(&profile, true, &mut journal)
        .await
        .expect("Dry run start failed");
    assert!(backend.is_running().await);

    // Verify journal recorded CreatedInterface
    assert_eq!(journal.actions.len(), 1);
    match &journal.actions[0] {
        JournalAction::CreatedInterface { name } => assert_eq!(name, "awg-dry0"),
        other => panic!("Expected CreatedInterface, got {other:?}"),
    }

    // Verify stats
    let stats = backend.stats().await.expect("Failed to get stats");
    assert_eq!(stats.interface_name, "awg-dry0");
    assert_eq!(stats.endpoint, "203.0.113.10:39001");

    // Stop in dry_run mode
    backend
        .stop(true, &mut journal)
        .await
        .expect("Dry run stop failed");
    assert!(!backend.is_running().await);
}
