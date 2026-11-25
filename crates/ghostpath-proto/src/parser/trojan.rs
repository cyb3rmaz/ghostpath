use ghostpath_core::models::{NodeProtocol, ServerNode};
use ghostpath_core::{CoreError, Result};
use url::Url;

#[derive(Debug, Clone, Default)]
pub struct TrojanConfig {
    pub password: String,
    pub host: String,
    pub port: u16,
    pub security: String,
    pub transport: String,
    pub sni: Option<String>,
    pub alpn: Option<String>,
    pub path: Option<String>,
    pub name: String,
}

const MAX_TROJAN_URI_LEN: usize = 16_384;

pub fn parse_trojan_uri(raw: &str) -> Result<TrojanConfig> {
    let raw = raw.trim();
    if raw.len() > MAX_TROJAN_URI_LEN {
        return Err(CoreError::Config(
            "Trojan URI exceeds max safe length".into(),
        ));
    }
    if !raw.starts_with("trojan://") {
        return Err(CoreError::Config("URI must start with trojan://".into()));
    }

    let parsed = Url::parse(raw)
        .map_err(|e| CoreError::Config(format!("Invalid Trojan URL format: {e}")))?;

    let password = parsed.username().to_string();
    if password.is_empty() || password.chars().any(char::is_control) {
        return Err(CoreError::Config(
            "Trojan URI missing or invalid password".into(),
        ));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| CoreError::Config("Trojan URI missing host".into()))?
        .to_string();
    if host.is_empty() || host.chars().any(char::is_control) {
        return Err(CoreError::Config("Trojan URI invalid host".into()));
    }

    let port = parsed
        .port()
        .filter(|&p| p > 0)
        .ok_or_else(|| CoreError::Config("Trojan URI missing or invalid port".into()))?;

    let name = parsed
        .fragment()
        .map(|f| {
            percent_encoding::percent_decode_str(f)
                .decode_utf8_lossy()
                .into_owned()
        })
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("{host}:{port}"));

    let mut security = "tls".to_string();
    let mut transport = "tcp".to_string();
    let mut sni = None;
    let mut alpn = None;
    let mut path = None;

    for (key, val) in parsed.query_pairs() {
        match key.as_ref() {
            "security" => security = val.into_owned(),
            "type" => transport = val.into_owned(),
            "sni" => sni = Some(val.into_owned()),
            "alpn" => alpn = Some(val.into_owned()),
            "path" => path = Some(val.into_owned()),
            _ => {}
        }
    }

    Ok(TrojanConfig {
        password,
        host,
        port,
        security,
        transport,
        sni,
        alpn,
        path,
        name,
    })
}

impl TrojanConfig {
    pub fn to_server_node(&self, raw: &str) -> ServerNode {
        ServerNode {
            id: format!("node-{}", uuid::Uuid::new_v4().simple()),
            name: self.name.clone(),
            country_code: "DE".to_string(),
            city: "Unknown".to_string(),
            country: "Unknown".to_string(),
            protocol: NodeProtocol::Trojan,
            transport: self.transport.to_uppercase(),
            security: self.security.to_uppercase(),
            host: self.host.clone(),
            port: self.port,
            latency_ms: None,
            pinging: Some(false),
            load_percent: 15,
            subscription_id: None,
            raw_link: Some(raw.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_trojan() {
        let uri =
            "trojan://secret123@185.158.248.31:443?security=tls&sni=myhost.com#Frankfurt%20DE";
        let config = parse_trojan_uri(uri).unwrap();
        assert_eq!(config.password, "secret123");
        assert_eq!(config.host, "185.158.248.31");
        assert_eq!(config.port, 443);
        assert_eq!(config.sni.as_deref(), Some("myhost.com"));
        assert_eq!(config.name, "Frankfurt DE");
    }
}
