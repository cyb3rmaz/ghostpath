use ghostpath_proto::amnezia::{
    AwgInterfaceConfig, AwgObfuscationConfig, AwgPeerConfig, AwgProfileConfig,
};

#[test]
fn test_awg_obfuscation_validation_success() {
    let mut obf = AwgObfuscationConfig::default();
    obf.jc = Some(4);
    obf.jmin = Some(40);
    obf.jmax = Some(70);
    obf.s1 = Some(15);
    obf.s2 = Some(50);
    obf.h1 = Some(1234567891);
    obf.h2 = Some(2234567892);
    obf.h3 = Some(3234567893);
    obf.h4 = Some(4234567894);

    assert!(obf.validate().is_ok());
    assert!(obf.is_obfuscated());
}

#[test]
fn test_awg_obfuscation_validation_jmin_greater_than_jmax() {
    let mut obf = AwgObfuscationConfig::default();
    obf.jmin = Some(100);
    obf.jmax = Some(50);

    let err = obf.validate().unwrap_err();
    assert!(err.to_string().contains("Jmin cannot be greater than Jmax"));
}

#[test]
fn test_awg_obfuscation_validation_duplicate_headers() {
    let mut obf = AwgObfuscationConfig::default();
    obf.h1 = Some(1000);
    obf.h2 = Some(1000); // duplicate

    let err = obf.validate().unwrap_err();
    assert!(err.to_string().contains("H1..H4 magic headers must be pairwise distinct"));
}

#[test]
fn test_awg_profile_validation() {
    let profile = AwgProfileConfig {
        id: "test-awg-1".to_string(),
        name: "Test AWG".to_string(),
        interface: AwgInterfaceConfig {
            private_key: "a".repeat(44), // Base64 32 bytes
            address: vec!["10.8.0.2/24".to_string()],
            dns: vec!["1.1.1.1".to_string()],
            mtu: Some(1420),
            obfuscation: AwgObfuscationConfig {
                jc: Some(4),
                jmin: Some(40),
                jmax: Some(70),
                s1: Some(15),
                s2: Some(50),
                s3: None,
                s4: None,
                h1: Some(100),
                h2: Some(200),
                h3: Some(300),
                h4: Some(400),
                i1: None,
                i2: None,
                i3: None,
                i4: None,
                i5: None,
            },
        },
        peer: AwgPeerConfig {
            public_key: "b".repeat(44),
            preshared_key: None,
            endpoint: "203.0.113.10:39001".to_string(),
            allowed_ips: vec!["0.0.0.0/0".to_string()],
            persistent_keepalive: Some(25),
        },
    };

    assert!(profile.validate().is_ok());
    assert!(profile.is_amnezia());
}
