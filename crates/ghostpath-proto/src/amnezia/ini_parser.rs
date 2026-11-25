use super::model::{
    AmneziaError, AwgInterfaceConfig, AwgObfuscationConfig, AwgPeerConfig, AwgProfileConfig,
};
use uuid::Uuid;

enum IniSection {
    None,
    Interface,
    Peer,
    Other,
}

/// Maximum INI payload accepted (512 KiB) to prevent memory/CPU exhaustion.
pub const MAX_INI_BYTES: usize = 512 * 1024;
/// Maximum number of lines parsed from an INI file.
pub const MAX_INI_LINES: usize = 10_000;

/// Parses an AmneziaWG or WireGuard INI-style configuration string.
pub fn parse_awg_conf(input: &str, profile_name: &str) -> Result<AwgProfileConfig, AmneziaError> {
    if input.len() > MAX_INI_BYTES {
        return Err(AmneziaError::DecompressionSizeExceeded {
            size: input.len(),
            max_size: MAX_INI_BYTES,
        });
    }

    let mut current_section = IniSection::None;

    let mut private_key = None;
    let mut addresses = Vec::new();
    let mut dns_servers = Vec::new();
    let mut mtu = None;
    let mut obfuscation = AwgObfuscationConfig::default();

    let mut public_key = None;
    let mut preshared_key = None;
    let mut endpoint = None;
    let mut allowed_ips = Vec::new();
    let mut persistent_keepalive = None;

    let mut saw_interface = false;
    let mut saw_peer = false;

    for (line_num, raw_line) in input.lines().enumerate() {
        if line_num >= MAX_INI_LINES {
            return Err(AmneziaError::InvalidIni(format!(
                "INI line count exceeds safety limit of {MAX_INI_LINES}"
            )));
        }
        let line = raw_line.trim();

        // Skip blank lines and comments (# or ;)
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // Section headers
        if line.starts_with('[') && line.ends_with(']') {
            let section_name = line[1..line.len() - 1].trim().to_lowercase();
            match section_name.as_str() {
                "interface" => {
                    current_section = IniSection::Interface;
                    saw_interface = true;
                }
                "peer" => {
                    current_section = IniSection::Peer;
                    saw_peer = true;
                }
                _ => {
                    current_section = IniSection::Other;
                }
            }
            continue;
        }

        // Key = Value pairs
        let Some((key, val)) = line.split_once('=') else {
            return Err(AmneziaError::InvalidIni(format!(
                "Line {}: Expected 'Key = Value', got '{}'",
                line_num + 1,
                line
            )));
        };

        let key = key.trim().to_lowercase();
        let val = val.trim();

        match current_section {
            IniSection::Interface => match key.as_str() {
                "privatekey" => private_key = Some(val.to_string()),
                "address" => {
                    for addr in val.split(',') {
                        let a = addr.trim();
                        if !a.is_empty() {
                            addresses.push(a.to_string());
                        }
                    }
                }
                "dns" => {
                    for d in val.split(',') {
                        let d = d.trim();
                        if !d.is_empty() {
                            dns_servers.push(d.to_string());
                        }
                    }
                }
                "mtu" => {
                    let m = val.parse::<u16>().map_err(|_| {
                        AmneziaError::InvalidIni(format!("Invalid MTU number: '{val}'"))
                    })?;
                    mtu = Some(m);
                }
                "jc" => {
                    let v = val.parse::<u16>().map_err(|_| {
                        AmneziaError::InvalidObfuscation(format!("Invalid Jc number: '{val}'"))
                    })?;
                    obfuscation.jc = Some(v);
                }
                "jmin" => {
                    let v = val.parse::<u16>().map_err(|_| {
                        AmneziaError::InvalidObfuscation(format!("Invalid Jmin number: '{val}'"))
                    })?;
                    obfuscation.jmin = Some(v);
                }
                "jmax" => {
                    let v = val.parse::<u16>().map_err(|_| {
                        AmneziaError::InvalidObfuscation(format!("Invalid Jmax number: '{val}'"))
                    })?;
                    obfuscation.jmax = Some(v);
                }
                "s1" => {
                    let v = val.parse::<u16>().map_err(|_| {
                        AmneziaError::InvalidObfuscation(format!("Invalid S1 number: '{val}'"))
                    })?;
                    obfuscation.s1 = Some(v);
                }
                "s2" => {
                    let v = val.parse::<u16>().map_err(|_| {
                        AmneziaError::InvalidObfuscation(format!("Invalid S2 number: '{val}'"))
                    })?;
                    obfuscation.s2 = Some(v);
                }
                "s3" => {
                    let v = val.parse::<u16>().map_err(|_| {
                        AmneziaError::InvalidObfuscation(format!("Invalid S3 number: '{val}'"))
                    })?;
                    obfuscation.s3 = Some(v);
                }
                "s4" => {
                    let v = val.parse::<u16>().map_err(|_| {
                        AmneziaError::InvalidObfuscation(format!("Invalid S4 number: '{val}'"))
                    })?;
                    obfuscation.s4 = Some(v);
                }
                "h1" => {
                    let v = val.parse::<u32>().map_err(|_| {
                        AmneziaError::InvalidObfuscation(format!("Invalid H1 number: '{val}'"))
                    })?;
                    obfuscation.h1 = Some(v);
                }
                "h2" => {
                    let v = val.parse::<u32>().map_err(|_| {
                        AmneziaError::InvalidObfuscation(format!("Invalid H2 number: '{val}'"))
                    })?;
                    obfuscation.h2 = Some(v);
                }
                "h3" => {
                    let v = val.parse::<u32>().map_err(|_| {
                        AmneziaError::InvalidObfuscation(format!("Invalid H3 number: '{val}'"))
                    })?;
                    obfuscation.h3 = Some(v);
                }
                "h4" => {
                    let v = val.parse::<u32>().map_err(|_| {
                        AmneziaError::InvalidObfuscation(format!("Invalid H4 number: '{val}'"))
                    })?;
                    obfuscation.h4 = Some(v);
                }
                "i1" => obfuscation.i1 = Some(val.to_string()),
                "i2" => obfuscation.i2 = Some(val.to_string()),
                "i3" => obfuscation.i3 = Some(val.to_string()),
                "i4" => obfuscation.i4 = Some(val.to_string()),
                "i5" => obfuscation.i5 = Some(val.to_string()),
                _ => {} // Ignore unrecognized interface keys
            },
            IniSection::Peer => match key.as_str() {
                "publickey" => public_key = Some(val.to_string()),
                "presharedkey" => preshared_key = Some(val.to_string()),
                "endpoint" => endpoint = Some(val.to_string()),
                "allowedips" => {
                    for ip in val.split(',') {
                        let i = ip.trim();
                        if !i.is_empty() {
                            allowed_ips.push(i.to_string());
                        }
                    }
                }
                "persistentkeepalive" => {
                    let k = val.parse::<u16>().map_err(|_| {
                        AmneziaError::InvalidIni(format!(
                            "Invalid PersistentKeepalive number: '{val}'"
                        ))
                    })?;
                    persistent_keepalive = Some(k);
                }
                _ => {} // Ignore unrecognized peer keys
            },
            IniSection::None | IniSection::Other => {}
        }
    }

    if !saw_interface {
        return Err(AmneziaError::MissingRequiredField(
            "[Interface] section".into(),
        ));
    }
    if !saw_peer {
        return Err(AmneziaError::MissingRequiredField("[Peer] section".into()));
    }

    let private_key =
        private_key.ok_or_else(|| AmneziaError::MissingRequiredField("PrivateKey".into()))?;
    let public_key =
        public_key.ok_or_else(|| AmneziaError::MissingRequiredField("PublicKey".into()))?;
    let endpoint = endpoint.ok_or_else(|| AmneziaError::MissingRequiredField("Endpoint".into()))?;

    if allowed_ips.is_empty() {
        allowed_ips.push("0.0.0.0/0".to_string());
        allowed_ips.push("::/0".to_string());
    }

    let profile = AwgProfileConfig {
        id: format!("awg-{}", Uuid::new_v4().simple()),
        name: if profile_name.trim().is_empty() {
            "AmneziaWG Tunnel".to_string()
        } else {
            profile_name.to_string()
        },
        interface: AwgInterfaceConfig {
            private_key,
            address: addresses,
            dns: dns_servers,
            mtu,
            obfuscation,
        },
        peer: AwgPeerConfig {
            public_key,
            preshared_key,
            endpoint,
            allowed_ips,
            persistent_keepalive,
        },
    };

    profile.validate()?;
    Ok(profile)
}

/// Exports an `AwgProfileConfig` into standard AmneziaWG / WireGuard INI format.
pub fn export_awg_conf(profile: &AwgProfileConfig) -> String {
    let mut out = String::with_capacity(512);

    out.push_str("[Interface]\n");
    if !profile.interface.address.is_empty() {
        out.push_str(&format!(
            "Address = {}\n",
            profile.interface.address.join(", ")
        ));
    }
    out.push_str(&format!("PrivateKey = {}\n", profile.interface.private_key));

    if !profile.interface.dns.is_empty() {
        out.push_str(&format!("DNS = {}\n", profile.interface.dns.join(", ")));
    }
    if let Some(mtu) = profile.interface.mtu {
        out.push_str(&format!("MTU = {mtu}\n"));
    }

    // Obfuscation parameters
    let obf = &profile.interface.obfuscation;
    if let Some(jc) = obf.jc {
        out.push_str(&format!("Jc = {jc}\n"));
    }
    if let Some(jmin) = obf.jmin {
        out.push_str(&format!("Jmin = {jmin}\n"));
    }
    if let Some(jmax) = obf.jmax {
        out.push_str(&format!("Jmax = {jmax}\n"));
    }
    if let Some(s1) = obf.s1 {
        out.push_str(&format!("S1 = {s1}\n"));
    }
    if let Some(s2) = obf.s2 {
        out.push_str(&format!("S2 = {s2}\n"));
    }
    if let Some(s3) = obf.s3 {
        out.push_str(&format!("S3 = {s3}\n"));
    }
    if let Some(s4) = obf.s4 {
        out.push_str(&format!("S4 = {s4}\n"));
    }
    if let Some(h1) = obf.h1 {
        out.push_str(&format!("H1 = {h1}\n"));
    }
    if let Some(h2) = obf.h2 {
        out.push_str(&format!("H2 = {h2}\n"));
    }
    if let Some(h3) = obf.h3 {
        out.push_str(&format!("H3 = {h3}\n"));
    }
    if let Some(h4) = obf.h4 {
        out.push_str(&format!("H4 = {h4}\n"));
    }
    if let Some(ref i1) = obf.i1 {
        out.push_str(&format!("I1 = {i1}\n"));
    }
    if let Some(ref i2) = obf.i2 {
        out.push_str(&format!("I2 = {i2}\n"));
    }
    if let Some(ref i3) = obf.i3 {
        out.push_str(&format!("I3 = {i3}\n"));
    }
    if let Some(ref i4) = obf.i4 {
        out.push_str(&format!("I4 = {i4}\n"));
    }
    if let Some(ref i5) = obf.i5 {
        out.push_str(&format!("I5 = {i5}\n"));
    }

    out.push_str("\n[Peer]\n");
    out.push_str(&format!("PublicKey = {}\n", profile.peer.public_key));
    if let Some(ref psk) = profile.peer.preshared_key {
        out.push_str(&format!("PresharedKey = {psk}\n"));
    }
    out.push_str(&format!("Endpoint = {}\n", profile.peer.endpoint));
    if !profile.peer.allowed_ips.is_empty() {
        out.push_str(&format!(
            "AllowedIPs = {}\n",
            profile.peer.allowed_ips.join(", ")
        ));
    }
    if let Some(keepalive) = profile.peer.persistent_keepalive {
        out.push_str(&format!("PersistentKeepalive = {keepalive}\n"));
    }

    out
}
