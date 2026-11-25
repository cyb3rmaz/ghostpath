use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use ghostpath_core::models::{NodeProtocol, ServerNode};
use ghostpath_core::{CoreError, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct VmessRawJson {
    #[serde(default)]
    pub v: String,
    #[serde(default)]
    pub ps: String,
    pub add: String,
    pub port: serde_json::Value,
    pub id: String,
    #[serde(default)]
    pub aid: serde_json::Value,
    #[serde(default)]
    pub net: String,
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub tls: String,
    #[serde(default)]
    pub sni: String,
}

#[derive(Debug, Clone)]
pub struct VmessConfig {
    pub uuid: String,
    pub host: String,
    pub port: u16,
    pub security: String,
    pub transport: String,
    pub sni: Option<String>,
    pub path: Option<String>,
    pub name: String,
}

const MAX_VMESS_URI_LEN: usize = 16_384;

pub fn parse_vmess_uri(raw: &str) -> Result<VmessConfig> {
    let raw = raw.trim();
    if raw.len() > MAX_VMESS_URI_LEN {
        return Err(CoreError::Config(
            "VMess URI exceeds max safe length".into(),
        ));
    }
    if !raw.starts_with("vmess://") {
        return Err(CoreError::Config("URI must start with vmess://".into()));
    }

    let b64 = &raw["vmess://".len()..];
    let decoded_bytes = BASE64
        .decode(b64.as_bytes())
        .map_err(|e| CoreError::Config(format!("Failed to base64 decode VMess URI: {e}")))?;

    let json_val: VmessRawJson = serde_json::from_slice(&decoded_bytes)
        .map_err(|e| CoreError::Config(format!("Invalid VMess JSON: {e}")))?;

    if json_val.add.trim().is_empty() || json_val.add.chars().any(char::is_control) {
        return Err(CoreError::Config(
            "VMess URI missing or invalid host".into(),
        ));
    }
    if json_val.id.trim().is_empty() || json_val.id.chars().any(char::is_control) {
        return Err(CoreError::Config(
            "VMess URI missing or invalid UUID".into(),
        ));
    }

    let port: u16 = match json_val.port {
        serde_json::Value::Number(n) => n
            .as_u64()
            .and_then(|p| u16::try_from(p).ok())
            .filter(|&p| p > 0)
            .ok_or_else(|| CoreError::Config("Invalid VMess port".into()))?,
        serde_json::Value::String(s) => s
            .parse::<u16>()
            .ok()
            .filter(|&p| p > 0)
            .ok_or_else(|| CoreError::Config("Invalid VMess port string".into()))?,
        _ => return Err(CoreError::Config("Missing VMess port".into())),
    };

    let name = if json_val.ps.is_empty() {
        format!("{}:{}", json_val.add, port)
    } else {
        json_val.ps
    };

    let security = if json_val.tls == "tls" {
        "tls".to_string()
    } else {
        "none".to_string()
    };

    let transport = if json_val.net.is_empty() {
        "tcp".to_string()
    } else {
        json_val.net
    };

    let sni = if !json_val.sni.is_empty() {
        Some(json_val.sni)
    } else if !json_val.host.is_empty() {
        Some(json_val.host)
    } else {
        None
    };

    let path = if !json_val.path.is_empty() {
        Some(json_val.path)
    } else {
        None
    };

    Ok(VmessConfig {
        uuid: json_val.id,
        host: json_val.add,
        port,
        security,
        transport,
        sni,
        path,
        name,
    })
}

impl VmessConfig {
    pub fn to_server_node(&self, raw: &str) -> ServerNode {
        ServerNode {
            id: format!("node-{}", uuid::Uuid::new_v4().simple()),
            name: self.name.clone(),
            country_code: "US".to_string(),
            city: "Unknown".to_string(),
            country: "Unknown".to_string(),
            protocol: NodeProtocol::VMess,
            transport: self.transport.to_uppercase(),
            security: self.security.to_uppercase(),
            host: self.host.clone(),
            port: self.port,
            latency_ms: None,
            pinging: Some(false),
            load_percent: 20,
            subscription_id: None,
            raw_link: Some(raw.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vmess() {
        let json_str = r#"{"v":"2","ps":"Tokyo JP-01","add":"192.0.2.1","port":10086,"id":"b831381d-6324-4d53-ad4f-8cda48b30811","aid":0,"net":"ws","type":"none","host":"example.jp","path":"/vmess","tls":"tls"}"#;
        let b64 = BASE64.encode(json_str);
        let uri = format!("vmess://{}", b64);

        let config = parse_vmess_uri(&uri).unwrap();
        assert_eq!(config.name, "Tokyo JP-01");
        assert_eq!(config.host, "192.0.2.1");
        assert_eq!(config.port, 10086);
        assert_eq!(config.transport, "ws");
        assert_eq!(config.security, "tls");
        assert_eq!(config.sni.as_deref(), Some("example.jp"));
        assert_eq!(config.path.as_deref(), Some("/vmess"));
    }
}
