use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::Url;

use crate::error::{CoreError, Result};

/// Validates a Linux network interface name (`1..=15` chars, no leading `-`, no slashes/spaces/control chars).
pub fn validate_iface_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 15 {
        return Err(CoreError::Config(format!(
            "Invalid interface name length: '{name}'"
        )));
    }
    if name.starts_with('-') || name == "." || name == ".." {
        return Err(CoreError::Config(format!(
            "Invalid interface name: '{name}'"
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(CoreError::Config(format!(
            "Interface name contains disallowed characters: '{name}'"
        )));
    }
    Ok(())
}

/// Validates an excluded interface glob (e.g. `docker0`, `br-*`, `virbr*`).
pub fn validate_iface_glob(pattern: &str) -> Result<()> {
    if pattern.is_empty() || pattern.len() > 32 {
        return Err(CoreError::Config(format!(
            "Invalid interface glob length: '{pattern}'"
        )));
    }
    if pattern.starts_with('-') {
        return Err(CoreError::Config(format!(
            "Interface pattern cannot start with '-': '{pattern}'"
        )));
    }
    if !pattern
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '*'))
    {
        return Err(CoreError::Config(format!(
            "Interface pattern contains disallowed characters: '{pattern}'"
        )));
    }
    Ok(())
}

/// Validates an IPv4 or IPv6 address string without whitespace or flags.
pub fn validate_ip_addr(addr: &str) -> Result<IpAddr> {
    if addr.starts_with('-') || addr.chars().any(char::is_whitespace) {
        return Err(CoreError::Config(format!("Invalid IP address: '{addr}'")));
    }
    addr.parse::<IpAddr>()
        .map_err(|_| CoreError::Config(format!("Invalid IP address: '{addr}'")))
}

/// Validates an IPv4/IPv6 address optionally paired with a prefix length (`ip/prefix`).
pub fn validate_cidr(cidr: &str) -> Result<()> {
    if cidr.starts_with('-') || cidr.chars().any(char::is_whitespace) {
        return Err(CoreError::Config(format!("Invalid CIDR: '{cidr}'")));
    }
    let Some((ip_part, prefix_part)) = cidr.split_once('/') else {
        validate_ip_addr(cidr)?;
        return Ok(());
    };
    let ip: IpAddr = ip_part
        .parse()
        .map_err(|_| CoreError::Config(format!("Invalid CIDR IP: '{cidr}'")))?;
    let prefix: u8 = prefix_part
        .parse()
        .map_err(|_| CoreError::Config(format!("Invalid CIDR prefix: '{cidr}'")))?;
    let max_prefix = match ip {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    };
    if prefix > max_prefix {
        return Err(CoreError::Config(format!(
            "CIDR prefix out of range (max {max_prefix}): '{cidr}'"
        )));
    }
    Ok(())
}

/// Validates `host:port` or `[ipv6]:port` endpoints for CLI/UAPI safety.
pub fn validate_host_port(endpoint: &str) -> Result<(String, u16)> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('-')
        || trimmed.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(CoreError::Config("Invalid endpoint syntax".to_string()));
    }
    let (host, port_str) = if let Some(rest) = trimmed.strip_prefix('[') {
        let (h, tail) = rest
            .split_once("]:")
            .ok_or_else(|| CoreError::Config("Invalid bracketed IPv6 endpoint".to_string()))?;
        (h, tail)
    } else {
        trimmed
            .rsplit_once(':')
            .ok_or_else(|| CoreError::Config("Endpoint missing ':port'".to_string()))?
    };

    if host.is_empty() || host.starts_with('-') {
        return Err(CoreError::Config("Invalid endpoint host".to_string()));
    }
    let port: u16 = port_str
        .parse()
        .map_err(|_| CoreError::Config("Invalid endpoint port".to_string()))?;
    if port == 0 {
        return Err(CoreError::Config("Endpoint port cannot be 0".to_string()));
    }
    Ok((host.to_string(), port))
}

/// Validates MTU in a safe operational range (`576..=9000`).
pub fn validate_mtu(mtu: u16) -> Result<u16> {
    if !(576..=9000).contains(&mtu) {
        return Err(CoreError::Config(format!(
            "MTU {mtu} out of allowed range 576..=9000"
        )));
    }
    Ok(mtu)
}

/// Validates unprivileged local proxy listening port (`1024..=65535`).
pub fn validate_local_port(port: u16) -> Result<u16> {
    if port < 1024 {
        return Err(CoreError::Config(format!(
            "Local port {port} must be in unprivileged range 1024..=65535"
        )));
    }
    Ok(port)
}

/// Validates `tun_stack` value against supported `sing-box` stacks.
pub fn validate_tun_stack(stack: &str) -> Result<()> {
    match stack {
        "system" | "gvisor" | "mixed" => Ok(()),
        _ => Err(CoreError::Config(format!(
            "Unsupported TUN stack '{stack}'; expected system, gvisor, or mixed"
        ))),
    }
}

/// Validates a configured DNS resolver endpoint (IP address or `https://`/`tls://` endpoint).
pub fn validate_dns_endpoint(dns: &str) -> Result<()> {
    let trimmed = dns.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('-')
        || trimmed.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(CoreError::Config("Invalid DNS endpoint".to_string()));
    }
    if trimmed.parse::<IpAddr>().is_ok() {
        return Ok(());
    }
    let parsed = Url::parse(trimmed)
        .map_err(|_| CoreError::Config("Invalid DNS resolver URL or IP".to_string()))?;
    if !matches!(parsed.scheme(), "https" | "tls") {
        return Err(CoreError::Config(
            "DNS URL scheme must be https:// or tls://".to_string(),
        ));
    }
    if parsed.host_str().is_none() {
        return Err(CoreError::Config("DNS URL missing host".to_string()));
    }
    Ok(())
}

/// Validates a line-oriented UAPI value (must not contain CR, LF, NUL, or `=`).
pub fn validate_uapi_value(val: &str) -> Result<()> {
    if val
        .chars()
        .any(|c| c == '\n' || c == '\r' || c == '\0' || c == '=' || c.is_control())
    {
        return Err(CoreError::Config(
            "UAPI parameter contains forbidden control or separator characters".to_string(),
        ));
    }
    Ok(())
}

/// Strips control characters and Unicode BiDi override codepoints from untrusted strings.
#[must_use]
pub fn sanitize_display_text(input: &str, max_len: usize) -> String {
    input
        .chars()
        .filter(|&c| {
            !c.is_control()
                && !matches!(
                    c,
                    '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}'
                )
        })
        .take(max_len)
        .collect()
}

/// Redacts credentials, path tokens, and query strings when logging URLs.
#[must_use]
pub fn redact_url_for_log(raw: &str) -> String {
    match Url::parse(raw.trim()) {
        Ok(u) => {
            let host = u.host_str().unwrap_or("<unknown>");
            match u.port() {
                Some(p) => format!("{}://{}:{}/***", u.scheme(), host, p),
                None => format!("{}://{}/***", u.scheme(), host),
            }
        }
        Err(_) => "<invalid-url>".to_string(),
    }
}

/// Returns `true` only if `ip` is globally routable (blocking loopback, private, link-local,
/// metadata `169.254.0.0/16`, CGNAT `100.64.0.0/10`, multicast, documentation, ULA, etc.).
#[must_use]
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_ipv4(v4),
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_public_ipv4(mapped);
            }
            is_public_ipv6(v6)
        }
    }
}

fn is_public_ipv4(v4: Ipv4Addr) -> bool {
    let octets = v4.octets();
    let is_cgnat = octets[0] == 100 && (octets[1] & 0xC0) == 64; // 100.64.0.0/10
    let is_benchmark = octets[0] == 198 && (octets[1] & 0xFE) == 18; // 198.18.0.0/15
    let is_ietf_prot = octets[0] == 192 && octets[1] == 0 && octets[2] == 0; // 192.0.0.0/24
    let is_future_use = (octets[0] & 0xF0) == 240; // 240.0.0.0/4

    !(v4.is_unspecified()
        || v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_multicast()
        || v4.is_broadcast()
        || v4.is_documentation()
        || is_cgnat
        || is_benchmark
        || is_ietf_prot
        || is_future_use)
}

fn is_public_ipv6(v6: Ipv6Addr) -> bool {
    let segs = v6.segments();
    let is_ula = (segs[0] & 0xFE00) == 0xFC00; // fc00::/7
    let is_link_local = (segs[0] & 0xFFC0) == 0xFE80; // fe80::/10
    let is_doc = segs[0] == 0x2001 && segs[1] == 0x0DB8; // 2001:db8::/32
    let is_discard = segs[0] == 0x0100 && segs[1] == 0 && segs[2] == 0 && segs[3] == 0;

    !(v6.is_unspecified()
        || v6.is_loopback()
        || v6.is_multicast()
        || is_ula
        || is_link_local
        || is_doc
        || is_discard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_option_injection_and_bad_iface() {
        assert!(validate_iface_name("-force").is_err());
        assert!(validate_iface_name("ghost0\n").is_err());
        assert!(validate_iface_name("ghost0/1").is_err());
        assert!(validate_iface_name("awg-ghost0").is_ok());

        assert!(validate_iface_glob("-v").is_err());
        assert!(validate_iface_glob("br-*").is_ok());

        assert!(validate_cidr("-4").is_err());
        assert!(validate_cidr("10.8.0.2/24").is_ok());
        assert!(validate_cidr("10.8.0.2/33").is_err());

        assert!(validate_uapi_value("abc\nreplace_peers=true").is_err());
        assert!(validate_uapi_value("<b 0x1234>").is_ok());
    }

    #[test]
    fn test_ssrf_ip_classification() {
        assert!(!is_public_ip("127.0.0.1".parse().unwrap()));
        assert!(!is_public_ip("169.254.169.254".parse().unwrap()));
        assert!(!is_public_ip("10.0.0.1".parse().unwrap()));
        assert!(!is_public_ip("172.16.5.1".parse().unwrap()));
        assert!(!is_public_ip("192.168.1.1".parse().unwrap()));
        assert!(!is_public_ip("100.64.0.1".parse().unwrap()));
        assert!(!is_public_ip("::1".parse().unwrap()));
        assert!(!is_public_ip("fe80::1".parse().unwrap()));
        assert!(!is_public_ip("fc00::1".parse().unwrap()));
        assert!(!is_public_ip("::ffff:127.0.0.1".parse().unwrap()));
        assert!(is_public_ip("1.1.1.1".parse().unwrap()));
        assert!(is_public_ip("2606:4700:4700::1111".parse().unwrap()));
    }

    #[test]
    fn test_redact_url_and_display_sanitize() {
        let redacted =
            redact_url_for_log("https://user:pass@sub.example.com:8443/secret-token?a=1");
        assert_eq!(redacted, "https://sub.example.com:8443/***");
        let clean = sanitize_display_text("Node\x1b[31m\u{202E}DE-01\n", 64);
        assert_eq!(clean, "Node[31mDE-01");
    }
}
