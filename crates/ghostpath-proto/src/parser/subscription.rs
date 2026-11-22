use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use ghostpath_core::models::ServerNode;
use ghostpath_core::Result;

use super::ss::parse_ss_uri;
use super::trojan::parse_trojan_uri;
use super::vless::parse_vless_uri;
use super::vmess::parse_vmess_uri;

pub fn parse_subscription_content(content: &str) -> Result<Vec<ServerNode>> {
    let trimmed = content.trim();

    // 1. Try decoding as base64 list of URIs
    if let Ok(decoded_bytes) = BASE64.decode(trimmed.as_bytes()) {
        if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
            let nodes = parse_uri_list(&decoded_str);
            if !nodes.is_empty() {
                return Ok(nodes);
            }
        }
    }

    // 2. Try parsing directly as plain text newline-delimited list of URIs
    let nodes = parse_uri_list(trimmed);
    if !nodes.is_empty() {
        return Ok(nodes);
    }

    // 3. Try parsing as JSON (sing-box / xray profile)
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(outbounds) = json_val.get("outbounds").and_then(|v| v.as_array()) {
            let mut parsed_nodes = Vec::new();
            for out in outbounds {
                if let Some(tag) = out.get("tag").and_then(|t| t.as_str()) {
                    let server = out
                        .get("server")
                        .and_then(|s| s.as_str())
                        .unwrap_or("127.0.0.1");
                    let port = out
                        .get("server_port")
                        .and_then(|p| p.as_u64())
                        .unwrap_or(443) as u16;
                    let o_type = out.get("type").and_then(|t| t.as_str()).unwrap_or("vless");

                    let proto = match o_type {
                        "vless" => ghostpath_core::models::NodeProtocol::VLESS,
                        "vmess" => ghostpath_core::models::NodeProtocol::VMess,
                        "trojan" => ghostpath_core::models::NodeProtocol::Trojan,
                        "shadowsocks" => ghostpath_core::models::NodeProtocol::Shadowsocks,
                        _ => continue,
                    };

                    parsed_nodes.push(ServerNode {
                        id: format!("node-{}", uuid::Uuid::new_v4().simple()),
                        name: tag.to_string(),
                        country_code: "US".to_string(),
                        city: "Unknown".to_string(),
                        country: "Unknown".to_string(),
                        protocol: proto,
                        transport: "TCP".to_string(),
                        security: "TLS".to_string(),
                        host: server.to_string(),
                        port,
                        latency_ms: None,
                        pinging: Some(false),
                        load_percent: 10,
                        subscription_id: None,
                        raw_link: None,
                    });
                }
            }
            if !parsed_nodes.is_empty() {
                return Ok(parsed_nodes);
            }
        }
    }

    Ok(Vec::new())
}

fn parse_uri_list(text: &str) -> Vec<ServerNode> {
    let mut nodes = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("vless://") {
            if let Ok(cfg) = parse_vless_uri(line) {
                nodes.push(cfg.to_server_node(line));
            }
        } else if line.starts_with("vmess://") {
            if let Ok(cfg) = parse_vmess_uri(line) {
                nodes.push(cfg.to_server_node(line));
            }
        } else if line.starts_with("trojan://") {
            if let Ok(cfg) = parse_trojan_uri(line) {
                nodes.push(cfg.to_server_node(line));
            }
        } else if line.starts_with("ss://") {
            if let Ok(cfg) = parse_ss_uri(line) {
                nodes.push(cfg.to_server_node(line));
            }
        }
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subscription_base64() {
        let vless = "vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@195.181.162.88:443?security=reality#SE-01";
        let trojan = "trojan://secret@185.158.248.31:443#DE-01";
        let raw_list = format!("{}\n{}\n", vless, trojan);
        let b64 = BASE64.encode(raw_list);

        let nodes = parse_subscription_content(&b64).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, "SE-01");
        assert_eq!(nodes[1].name, "DE-01");
    }
}
