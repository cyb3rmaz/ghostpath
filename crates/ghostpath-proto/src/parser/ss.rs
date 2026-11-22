use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ghostpath_core::models::{NodeProtocol, ServerNode};
use ghostpath_core::{CoreError, Result};
use url::Url;

#[derive(Debug, Clone)]
pub struct ShadowsocksConfig {
    pub method: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub name: String,
}

pub fn parse_ss_uri(raw: &str) -> Result<ShadowsocksConfig> {
    let raw = raw.trim();
    if !raw.starts_with("ss://") {
        return Err(CoreError::Config("URI must start with ss://".into()));
    }

    let without_prefix = &raw["ss://".len()..];
    let (body, fragment) = match without_prefix.find('#') {
        Some(idx) => (&without_prefix[..idx], Some(&without_prefix[idx + 1..])),
        None => (without_prefix, None),
    };

    let name = fragment
        .map(|f| {
            percent_encoding::percent_decode_str(f)
                .decode_utf8_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "Shadowsocks Server".to_string());

    if let Some(at_idx) = body.find('@') {
        // Standard SIP002: base64(method:password)@host:port
        let userinfo_b64 = &body[..at_idx];
        let hostport = &body[at_idx + 1..];

        let decoded = decode_b64(userinfo_b64)
            .map_err(|e| CoreError::Config(format!("Invalid SS userinfo base64: {e}")))?;
        let userinfo_str = String::from_utf8(decoded)
            .map_err(|e| CoreError::Config(format!("Invalid UTF-8 in SS userinfo: {e}")))?;

        let (method, password) = userinfo_str
            .split_once(':')
            .ok_or_else(|| CoreError::Config("SS userinfo missing colon".into()))?;

        let (host, port_str) = hostport
            .rsplit_once(':')
            .ok_or_else(|| CoreError::Config("SS hostport missing colon".into()))?;
        let port: u16 = port_str
            .parse()
            .map_err(|_| CoreError::Config("Invalid SS port".into()))?;

        Ok(ShadowsocksConfig {
            method: method.to_string(),
            password: password.to_string(),
            host: host.to_string(),
            port,
            name,
        })
    } else {
        // Legacy: base64(method:password@host:port)
        let decoded = decode_b64(body)
            .map_err(|e| CoreError::Config(format!("Invalid legacy SS base64: {e}")))?;
        let full_str = String::from_utf8(decoded)
            .map_err(|e| CoreError::Config(format!("Invalid UTF-8 in legacy SS string: {e}")))?;

        let dummy_url = format!("ss://{}", full_str);
        let parsed = Url::parse(&dummy_url)
            .map_err(|e| CoreError::Config(format!("Invalid legacy SS URL: {e}")))?;

        let method = parsed.username().to_string();
        let password = parsed.password().unwrap_or_default().to_string();
        let host = parsed
            .host_str()
            .ok_or_else(|| CoreError::Config("Missing host in SS URI".into()))?
            .to_string();
        let port = parsed
            .port()
            .ok_or_else(|| CoreError::Config("Missing port in SS URI".into()))?;

        Ok(ShadowsocksConfig {
            method,
            password,
            host,
            port,
            name,
        })
    }
}

fn decode_b64(s: &str) -> std::result::Result<Vec<u8>, base64::DecodeError> {
    if let Ok(res) = BASE64.decode(s.as_bytes()) {
        return Ok(res);
    }
    URL_SAFE_NO_PAD.decode(s.as_bytes())
}

impl ShadowsocksConfig {
    pub fn to_server_node(&self, raw: &str) -> ServerNode {
        ServerNode {
            id: format!("node-{}", uuid::Uuid::new_v4().simple()),
            name: self.name.clone(),
            country_code: "US".to_string(),
            city: "Unknown".to_string(),
            country: "Unknown".to_string(),
            protocol: NodeProtocol::Shadowsocks,
            transport: "TCP".to_string(),
            security: self.method.clone(),
            host: self.host.clone(),
            port: self.port,
            latency_ms: None,
            pinging: Some(false),
            load_percent: 10,
            subscription_id: None,
            raw_link: Some(raw.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ss_sip002() {
        let userinfo = BASE64.encode("chacha20-ietf-poly1305:mysecretpassword");
        let uri = format!("ss://{}@198.51.100.25:8388#London%20UK", userinfo);

        let config = parse_ss_uri(&uri).unwrap();
        assert_eq!(config.method, "chacha20-ietf-poly1305");
        assert_eq!(config.password, "mysecretpassword");
        assert_eq!(config.host, "198.51.100.25");
        assert_eq!(config.port, 8388);
        assert_eq!(config.name, "London UK");
    }
}
