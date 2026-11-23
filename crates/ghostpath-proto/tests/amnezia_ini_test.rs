use ghostpath_proto::amnezia::{export_awg_conf, parse_awg_conf, AmneziaError};

#[test]
fn test_parse_full_amnezia_conf() {
    let raw = r#"
[Interface]
Address = 10.8.0.2/24, fd00:5180::2/64
PrivateKey = YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=
DNS = 1.1.1.1, 1.0.0.1
MTU = 1420
Jc = 4
Jmin = 40
Jmax = 70
S1 = 15
S2 = 50
S3 = 10
S4 = 20
H1 = 1234567891
H2 = 2234567892
H3 = 3234567893
H4 = 4234567894
I1 = <b 0xa1b2c3d4>

[Peer]
PublicKey = YmJiamJiamJiamJiamJiamJiamJiamJiamJiamJiamJi=
PresharedKey = Y2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2M=
Endpoint = 203.0.113.10:39001
AllowedIPs = 0.0.0.0/0, ::/0
PersistentKeepalive = 25
"#;

    let profile = parse_awg_conf(raw, "amnezia-v1").expect("Failed to parse Amnezia conf");
    assert!(profile.is_amnezia());
    assert_eq!(profile.interface.address.len(), 2);
    assert_eq!(profile.interface.dns.len(), 2);
    assert_eq!(profile.interface.mtu, Some(1420));
    assert_eq!(profile.interface.obfuscation.jc, Some(4));
    assert_eq!(profile.interface.obfuscation.jmin, Some(40));
    assert_eq!(profile.interface.obfuscation.jmax, Some(70));
    assert_eq!(profile.interface.obfuscation.s1, Some(15));
    assert_eq!(profile.interface.obfuscation.s2, Some(50));
    assert_eq!(profile.interface.obfuscation.s3, Some(10));
    assert_eq!(profile.interface.obfuscation.s4, Some(20));
    assert_eq!(profile.interface.obfuscation.h1, Some(1234567891));
    assert_eq!(profile.interface.obfuscation.h2, Some(2234567892));
    assert_eq!(profile.interface.obfuscation.h3, Some(3234567893));
    assert_eq!(profile.interface.obfuscation.h4, Some(4234567894));
    assert_eq!(profile.interface.obfuscation.i1.as_deref(), Some("<b 0xa1b2c3d4>"));

    assert_eq!(profile.peer.endpoint, "203.0.113.10:39001");
    assert_eq!(profile.peer.allowed_ips.len(), 2);
    assert_eq!(profile.peer.persistent_keepalive, Some(25));
    assert!(profile.peer.preshared_key.is_some());
}

#[test]
fn test_parse_plain_wireguard_conf() {
    let raw = r#"
[Interface]
Address = 10.0.0.2/32
PrivateKey = YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=
DNS = 9.9.9.9

[Peer]
PublicKey = YmJiamJiamJiamJiamJiamJiamJiamJiamJiamJiamJi=
Endpoint = 198.51.100.1:51820
AllowedIPs = 0.0.0.0/0
"#;

    let profile = parse_awg_conf(raw, "plain-wg").expect("Failed to parse plain WG conf");
    assert!(!profile.is_amnezia());
    assert_eq!(profile.interface.obfuscation.is_obfuscated(), false);
    assert_eq!(profile.interface.address, vec!["10.0.0.2/32"]);
    assert_eq!(profile.interface.dns, vec!["9.9.9.9"]);
    assert_eq!(profile.peer.endpoint, "198.51.100.1:51820");
}

#[test]
fn test_roundtrip_export_and_parse() {
    let raw = r#"
[Interface]
Address = 10.8.0.2/24
PrivateKey = YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=
DNS = 1.1.1.1
MTU = 1380
Jc = 8
Jmin = 50
Jmax = 80
S1 = 25
S2 = 60
H1 = 1111111111
H2 = 2222222222
H3 = 3333333333
H4 = 4141414141

[Peer]
PublicKey = YmJiamJiamJiamJiamJiamJiamJiamJiamJiamJiamJi=
Endpoint = 203.0.113.50:40000
AllowedIPs = 0.0.0.0/0
PersistentKeepalive = 30
"#;

    let parsed1 = parse_awg_conf(raw, "test-rt").unwrap();
    let exported = export_awg_conf(&parsed1);
    let parsed2 = parse_awg_conf(&exported, "test-rt-2").unwrap();

    assert_eq!(parsed1.interface.address, parsed2.interface.address);
    assert_eq!(parsed1.interface.private_key, parsed2.interface.private_key);
    assert_eq!(parsed1.interface.dns, parsed2.interface.dns);
    assert_eq!(parsed1.interface.mtu, parsed2.interface.mtu);
    assert_eq!(parsed1.interface.obfuscation, parsed2.interface.obfuscation);
    assert_eq!(parsed1.peer.public_key, parsed2.peer.public_key);
    assert_eq!(parsed1.peer.endpoint, parsed2.peer.endpoint);
    assert_eq!(parsed1.peer.allowed_ips, parsed2.peer.allowed_ips);
    assert_eq!(parsed1.peer.persistent_keepalive, parsed2.peer.persistent_keepalive);
}

#[test]
fn test_parse_missing_interface_or_peer() {
    let no_peer = "[Interface]\nPrivateKey = YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=\nAddress = 10.0.0.2/32\n";
    let res = parse_awg_conf(no_peer, "no-peer");
    assert!(res.is_err());
    match res.unwrap_err() {
        AmneziaError::MissingRequiredField(f) => assert_eq!(f, "[Peer] section"),
        err => panic!("Unexpected error: {:?}", err),
    }

    let no_interface = "[Peer]\nPublicKey = YmJiamJiamJiamJiamJiamJiamJiamJiamJiamJiamJi=\nEndpoint = 1.2.3.4:51820\n";
    let res = parse_awg_conf(no_interface, "no-interface");
    assert!(res.is_err());
    match res.unwrap_err() {
        AmneziaError::MissingRequiredField(f) => assert_eq!(f, "[Interface] section"),
        err => panic!("Unexpected error: {:?}", err),
    }
}
