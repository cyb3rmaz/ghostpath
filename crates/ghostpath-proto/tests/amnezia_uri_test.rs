use ghostpath_proto::amnezia::{
    encode_vpn_uri, parse_vpn_uri, AmneziaContainerType, AmneziaError, AmneziaServerConfig,
};

#[test]
fn test_roundtrip_vpn_uri() {
    let raw_awg_conf = r#"[Interface]
Address = 10.8.0.2/24
PrivateKey = YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=
DNS = 1.1.1.1
Jc = 4
Jmin = 40
Jmax = 70
S1 = 15
S2 = 50
H1 = 100
H2 = 200
H3 = 300
H4 = 400

[Peer]
PublicKey = YmJiamJiamJiamJiamJiamJiamJiamJiamJiamJiamJi=
Endpoint = 203.0.113.10:39001
AllowedIPs = 0.0.0.0/0
"#;

    let parsed_profile =
        ghostpath_proto::amnezia::parse_awg_conf(raw_awg_conf, "Test AWG").unwrap();

    let server = AmneziaServerConfig {
        server_name: "Synthetic Amnezia Server".to_string(),
        host_name: "203.0.113.10".to_string(),
        default_container: Some("amnezia-awg".to_string()),
        dns: vec!["1.1.1.1".to_string()],
        containers: vec![
            ghostpath_proto::amnezia::AmneziaContainer {
                container_name: "amnezia-awg".to_string(),
                container_type: AmneziaContainerType::Awg,
                is_supported: true,
                unsupported_reason: None,
                awg_profile: Some(parsed_profile),
                raw_json: serde_json::json!({
                    "port": "39001",
                    "transport_proto": "udp"
                }),
            },
            ghostpath_proto::amnezia::AmneziaContainer {
                container_name: "amnezia-openvpn-cloak".to_string(),
                container_type: AmneziaContainerType::OpenVpnCloak,
                is_supported: false,
                unsupported_reason: Some("OpenVPN over Cloak is not supported".to_string()),
                awg_profile: None,
                raw_json: serde_json::json!({ "port": "443" }),
            },
        ],
    };

    let vpn_uri = encode_vpn_uri(&server).expect("Failed to encode vpn uri");
    assert!(vpn_uri.starts_with("vpn://"));

    let decoded_server = parse_vpn_uri(&vpn_uri).expect("Failed to decode vpn uri");
    assert_eq!(decoded_server.server_name, "Synthetic Amnezia Server");
    assert_eq!(decoded_server.host_name, "203.0.113.10");
    assert_eq!(decoded_server.containers.len(), 2);

    let awg_container = decoded_server
        .containers
        .iter()
        .find(|c| c.container_type == AmneziaContainerType::Awg)
        .expect("awg container missing");
    assert!(awg_container.is_supported);
    assert!(awg_container.awg_profile.is_some());
    let profile = awg_container.awg_profile.as_ref().unwrap();
    assert!(profile.is_amnezia());
    assert_eq!(profile.interface.obfuscation.jc, Some(4));

    let cloak_container = decoded_server
        .containers
        .iter()
        .find(|c| c.container_type == AmneziaContainerType::OpenVpnCloak)
        .expect("cloak container missing");
    assert!(!cloak_container.is_supported);
    assert!(cloak_container.unsupported_reason.is_some());
}

#[test]
fn test_vpn_uri_zip_bomb_prevention() {
    // Construct a malicious payload claiming an uncompressed size of 100 MB
    let huge_size: u32 = 100 * 1024 * 1024;
    let mut fake_payload = Vec::new();
    fake_payload.extend_from_slice(&huge_size.to_be_bytes());
    fake_payload.extend_from_slice(&[0x78, 0x9c, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01]); // valid empty zlib

    use base64::prelude::*;
    let b64 = BASE64_URL_SAFE_NO_PAD.encode(&fake_payload);
    let malicious_uri = format!("vpn://{b64}");

    let err = parse_vpn_uri(&malicious_uri).unwrap_err();
    match err {
        AmneziaError::DecompressionSizeExceeded { size, max_size } => {
            assert_eq!(size, 100 * 1024 * 1024);
            assert_eq!(max_size, 2 * 1024 * 1024);
        }
        other => panic!("Expected DecompressionSizeExceeded, got {other:?}"),
    }
}

#[test]
fn test_invalid_vpn_uri() {
    assert!(parse_vpn_uri("invalid://something").is_err());
    assert!(parse_vpn_uri("vpn://not-valid-base64-???").is_err());
    assert!(parse_vpn_uri("vpn://").is_err());
}
