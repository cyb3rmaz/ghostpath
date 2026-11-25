use ghostpath_core::models::{NodeProtocol, ServerNode};
use ghostpath_core::{CoreError, Result};
use url::Url;

#[derive(Debug, Clone, Default)]
pub struct VlessConfig {
    pub uuid: String,
    pub host: String,
    pub port: u16,
    pub flow: Option<String>,
    pub security: String,
    pub transport: String,
    pub pbk: Option<String>,
    pub sid: Option<String>,
    pub fp: Option<String>,
    pub sni: Option<String>,
    pub path: Option<String>,
    pub service_name: Option<String>,
    pub name: String,
}

const MAX_VLESS_URI_LEN: usize = 16_384;

pub fn parse_vless_uri(raw: &str) -> Result<VlessConfig> {
    let raw = raw.trim();
    if raw.len() > MAX_VLESS_URI_LEN {
        return Err(CoreError::Config(
            "VLESS URI exceeds max safe length".into(),
        ));
    }
    if !raw.starts_with("vless://") {
        return Err(CoreError::Config("URI must start with vless://".into()));
    }

    let parsed =
        Url::parse(raw).map_err(|e| CoreError::Config(format!("Invalid VLESS URL format: {e}")))?;

    let uuid = parsed.username().to_string();
    if uuid.is_empty() || uuid.chars().any(char::is_control) {
        return Err(CoreError::Config(
            "VLESS URI missing or invalid UUID".into(),
        ));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| CoreError::Config("VLESS URI missing host".into()))?
        .to_string();
    if host.is_empty() || host.chars().any(char::is_control) {
        return Err(CoreError::Config("VLESS URI invalid host".into()));
    }

    let port = parsed
        .port()
        .filter(|&p| p > 0)
        .ok_or_else(|| CoreError::Config("VLESS URI missing or invalid port".into()))?;

    let name = parsed
        .fragment()
        .map(|f| {
            percent_encoding::percent_decode_str(f)
                .decode_utf8_lossy()
                .into_owned()
        })
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("{host}:{port}"));

    let mut flow = None;
    let mut security = "none".to_string();
    let mut transport = "tcp".to_string();
    let mut pbk = None;
    let mut sid = None;
    let mut fp = None;
    let mut sni = None;
    let mut path = None;
    let mut service_name = None;

    for (key, val) in parsed.query_pairs() {
        match key.as_ref() {
            "flow" => flow = Some(val.into_owned()),
            "security" => security = val.into_owned(),
            "type" => transport = val.into_owned(),
            "pbk" => pbk = Some(val.into_owned()),
            "sid" => sid = Some(val.into_owned()),
            "fp" => fp = Some(val.into_owned()),
            "sni" => sni = Some(val.into_owned()),
            "path" => path = Some(val.into_owned()),
            "serviceName" => service_name = Some(val.into_owned()),
            _ => {}
        }
    }

    Ok(VlessConfig {
        uuid,
        host,
        port,
        flow,
        security,
        transport,
        pbk,
        sid,
        fp,
        sni,
        path,
        service_name,
        name,
    })
}

impl VlessConfig {
    pub fn to_server_node(&self, raw: &str) -> ServerNode {
        let sec_display = if self.security == "reality" {
            "Reality".to_string()
        } else if self.security == "tls" {
            "TLS".to_string()
        } else {
            "None".to_string()
        };

        let trans_display = if let Some(ref flow) = self.flow {
            format!("{} · {}", self.transport.to_uppercase(), flow)
        } else {
            self.transport.to_uppercase()
        };

        ServerNode {
            id: format!("node-{}", uuid::Uuid::new_v4().simple()),
            name: self.name.clone(),
            country_code: "US".to_string(),
            city: "Unknown".to_string(),
            country: "Unknown".to_string(),
            protocol: NodeProtocol::VLESS,
            transport: trans_display,
            security: sec_display,
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
    fn test_parse_vless_reality() {
        let uri = "vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@195.181.162.88:443?type=tcp&security=reality&flow=xtls-rprx-vision&pbk=Z3b7K_sample&sid=1a2b&fp=chrome&sni=example.com#Stockholm%20SE-01";
        let config = parse_vless_uri(uri).unwrap();
        assert_eq!(config.uuid, "a83e9f14-7b21-4e92-9f41-01c9b2e83c10");
        assert_eq!(config.host, "195.181.162.88");
        assert_eq!(config.port, 443);
        assert_eq!(config.security, "reality");
        assert_eq!(config.flow.as_deref(), Some("xtls-rprx-vision"));
        assert_eq!(config.pbk.as_deref(), Some("Z3b7K_sample"));
        assert_eq!(config.name, "Stockholm SE-01");

        let node = config.to_server_node(uri);
        assert_eq!(node.name, "Stockholm SE-01");
        assert_eq!(node.protocol, NodeProtocol::VLESS);
    }
}
