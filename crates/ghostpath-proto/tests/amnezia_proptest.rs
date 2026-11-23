use ghostpath_proto::amnezia::{
    export_awg_conf, parse_awg_conf, parse_vpn_uri, AwgInterfaceConfig, AwgObfuscationConfig,
    AwgPeerConfig, AwgProfileConfig,
};
use proptest::prelude::*;

proptest! {
    // 1. Fuzz parse_awg_conf with completely arbitrary strings (must never panic)
    #[test]
    fn fuzz_parse_awg_conf_arbitrary_string(s in "\\PC*") {
        let _ = parse_awg_conf(&s, "fuzz-profile");
    }

    // 2. Fuzz parse_vpn_uri with arbitrary strings (must never panic)
    #[test]
    fn fuzz_parse_vpn_uri_arbitrary_string(s in "\\PC*") {
        let _ = parse_vpn_uri(&s);
    }

    // 3. Fuzz parse_vpn_uri with "vpn://" + arbitrary base64-like strings
    #[test]
    fn fuzz_parse_vpn_uri_prefixed(s in "[a-zA-Z0-9_\\-=]{0,200}") {
        let uri = format!("vpn://{s}");
        let _ = parse_vpn_uri(&uri);
    }

    // 4. Round-trip property test: generate valid AWG configurations, export them, re-parse them
    #[test]
    fn prop_awg_conf_roundtrip(
        jc in 0u16..20u16,
        jmin in 10u16..50u16,
        jmax in 50u16..100u16,
        s1 in 0u16..100u16,
        s2 in 0u16..100u16,
        h1 in 1000u32..2000u32,
        h2 in 2001u32..3000u32,
        h3 in 3001u32..4000u32,
        h4 in 4001u32..5000u32,
        port in 1024u16..65535u16,
        keepalive in 10u16..60u16,
    ) {
        let original = AwgProfileConfig {
            id: "prop-test".to_string(),
            name: "Prop Test Profile".to_string(),
            interface: AwgInterfaceConfig {
                private_key: "YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=".to_string(),
                address: vec!["10.8.0.2/24".to_string(), "fd00:5180::2/64".to_string()],
                dns: vec!["1.1.1.1".to_string(), "1.0.0.1".to_string()],
                mtu: Some(1420),
                obfuscation: AwgObfuscationConfig {
                    jc: Some(jc),
                    jmin: Some(jmin),
                    jmax: Some(jmax),
                    s1: Some(s1),
                    s2: Some(s2),
                    s3: None,
                    s4: None,
                    h1: Some(h1),
                    h2: Some(h2),
                    h3: Some(h3),
                    h4: Some(h4),
                    i1: None,
                    i2: None,
                    i3: None,
                    i4: None,
                    i5: None,
                },
            },
            peer: AwgPeerConfig {
                public_key: "YmJiamJiamJiamJiamJiamJiamJiamJiamJiamJiamJi=".to_string(),
                preshared_key: Some("Y2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2M=".to_string()),
                endpoint: format!("203.0.113.10:{port}"),
                allowed_ips: vec!["0.0.0.0/0".to_string(), "::/0".to_string()],
                persistent_keepalive: Some(keepalive),
            },
        };

        let exported = export_awg_conf(&original);
        let parsed = parse_awg_conf(&exported, "Prop Test Profile").expect("Failed to parse exported config");

        prop_assert_eq!(&parsed.interface.address, &original.interface.address);
        prop_assert_eq!(&parsed.interface.dns, &original.interface.dns);
        prop_assert_eq!(parsed.interface.mtu, original.interface.mtu);
        prop_assert_eq!(parsed.interface.obfuscation.jc, original.interface.obfuscation.jc);
        prop_assert_eq!(parsed.interface.obfuscation.jmin, original.interface.obfuscation.jmin);
        prop_assert_eq!(parsed.interface.obfuscation.jmax, original.interface.obfuscation.jmax);
        prop_assert_eq!(parsed.interface.obfuscation.s1, original.interface.obfuscation.s1);
        prop_assert_eq!(parsed.interface.obfuscation.s2, original.interface.obfuscation.s2);
        prop_assert_eq!(parsed.interface.obfuscation.h1, original.interface.obfuscation.h1);
        prop_assert_eq!(parsed.interface.obfuscation.h2, original.interface.obfuscation.h2);
        prop_assert_eq!(parsed.interface.obfuscation.h3, original.interface.obfuscation.h3);
        prop_assert_eq!(parsed.interface.obfuscation.h4, original.interface.obfuscation.h4);
        prop_assert_eq!(&parsed.peer.endpoint, &original.peer.endpoint);
        prop_assert_eq!(&parsed.peer.allowed_ips, &original.peer.allowed_ips);
        prop_assert_eq!(parsed.peer.persistent_keepalive, original.peer.persistent_keepalive);
    }
}
