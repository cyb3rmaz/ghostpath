use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VpnLinkInfo {
    pub protocol: String,
    pub raw: String,
    pub name: Option<String>,
}

/// Extract and validate VPN protocol links (vless://, vmess://, trojan://, ss://)
pub fn parse_vpn_deep_link(uri_str: &str) -> Option<VpnLinkInfo> {
    let trimmed = uri_str.trim();
    let lower = trimmed.to_lowercase();

    let protocols = ["vless://", "vmess://", "trojan://", "ss://", "hysteria2://", "tuic://"];
    let matched_proto = protocols.iter().find(|p| lower.starts_with(*p))?;

    let proto_name = matched_proto.trim_end_matches("://").to_uppercase();

    // Extract optional name from URL fragment (#Name)
    let name = if let Some(hash_idx) = trimmed.find('#') {
        let fragment = &trimmed[hash_idx + 1..];
        if !fragment.is_empty() {
            // Replace percent-encoded hex sequences if present
            Some(fragment.replace("%20", " "))
        } else {
            None
        }
    } else {
        None
    };

    Some(VpnLinkInfo {
        protocol: proto_name,
        raw: trimmed.to_string(),
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vless_deep_link() {
        let uri = "vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@195.181.162.88:443?security=reality#Frankfurt-DE01";
        let info = parse_vpn_deep_link(uri).unwrap();
        assert_eq!(info.protocol, "VLESS");
        assert_eq!(info.name.as_deref(), Some("Frankfurt-DE01"));
        assert_eq!(info.raw, uri);
    }

    #[test]
    fn test_parse_vmess_deep_link() {
        let uri = "vmess://eyJhZGQiOiIxMjcuMC4wLjEiLCJwb3J0IjoxMDA4Nn0=";
        let info = parse_vpn_deep_link(uri).unwrap();
        assert_eq!(info.protocol, "VMESS");
        assert_eq!(info.name, None);
    }

    #[test]
    fn test_parse_invalid_deep_link() {
        assert!(parse_vpn_deep_link("https://example.com").is_none());
        assert!(parse_vpn_deep_link("ftp://ftp.example.com").is_none());
        assert!(parse_vpn_deep_link("").is_none());
    }
}
