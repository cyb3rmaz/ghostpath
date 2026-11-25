use ghostpath_core::models::{NodeProtocol, RoutingRule, RuleAction, RuleType, ServerNode};
use ghostpath_core::rules::default_routing_rules;
use ghostpath_core::{CoreError, Result};
use serde_json::{json, Value};

use crate::parser::ss::parse_ss_uri;
use crate::parser::trojan::parse_trojan_uri;
use crate::parser::vless::parse_vless_uri;
use crate::parser::vmess::parse_vmess_uri;

pub const GHOST_FWMARK: u32 = 0x4750; // 18256

pub struct SingBoxConfigOptions {
    pub socks_port: u16,
    pub http_port: u16,
    pub routing_mark: u32,
    pub rules: Vec<RoutingRule>,
    /// Optional TUN device interface name (e.g. `Some("ghost0")`) when running in TUN mode.
    pub tun_interface: Option<String>,
    pub tun_mtu: u16,
    pub tun_stack: String,
}

impl Default for SingBoxConfigOptions {
    fn default() -> Self {
        Self {
            socks_port: 10808,
            http_port: 10809,
            routing_mark: GHOST_FWMARK,
            rules: default_routing_rules(),
            tun_interface: None,
            tun_mtu: 1420,
            tun_stack: "system".to_string(),
        }
    }
}

pub fn generate_singbox_config(node: &ServerNode, options: &SingBoxConfigOptions) -> Result<Value> {
    let outbound = build_outbound(node, options.routing_mark)?;

    let mut route_rules = Vec::new();
    // Honor caller-supplied rules literally: never silently inject .ir direct bypasses when empty.
    for rule in &options.rules {
        if !rule.enabled {
            continue;
        }
        let target_outbound = match rule.action {
            RuleAction::Direct => "direct",
            RuleAction::Block => "block",
            RuleAction::Proxy => "proxy",
        };

        match rule.r#type {
            RuleType::Geosite => {
                route_rules.push(json!({
                    "geosite": [rule.pattern],
                    "outbound": target_outbound
                }));
            }
            RuleType::Geoip => {
                route_rules.push(json!({
                    "geoip": [rule.pattern],
                    "outbound": target_outbound
                }));
            }
            RuleType::Domain => {
                route_rules.push(json!({
                    "domain_suffix": [rule.pattern],
                    "outbound": target_outbound
                }));
            }
            RuleType::Cidr => {
                route_rules.push(json!({
                    "ip_cidr": [rule.pattern],
                    "outbound": target_outbound
                }));
            }
            RuleType::Process => {
                route_rules.push(json!({
                    "process_name": [rule.pattern],
                    "outbound": target_outbound
                }));
            }
        }
    }

    let mut inbound_tags = vec!["socks-in".to_string(), "mixed-in".to_string()];
    let mut inbounds = vec![
        json!({
            "type": "socks",
            "tag": "socks-in",
            "listen": "127.0.0.1",
            "listen_port": options.socks_port
        }),
        json!({
            "type": "mixed",
            "tag": "mixed-in",
            "listen": "127.0.0.1",
            "listen_port": options.http_port
        }),
    ];

    if let Some(ref tun_iface) = options.tun_interface {
        inbound_tags.push("tun-in".to_string());
        inbounds.push(json!({
            "type": "tun",
            "tag": "tun-in",
            "interface_name": tun_iface,
            "address": ["10.8.0.2/24"],
            "mtu": options.tun_mtu,
            "stack": options.tun_stack,
            "auto_route": false,
            "strict_route": false,
            "sniff": true
        }));
    }

    // Default fallback rule
    route_rules.push(json!({
        "inbound": inbound_tags,
        "outbound": "proxy"
    }));

    let config = json!({
        "log": {
            "level": "warn",
            "timestamp": true
        },
        "inbounds": inbounds,
        "outbounds": [
            outbound,
            {
                "type": "direct",
                "tag": "direct",
                "routing_mark": options.routing_mark
            },
            {
                "type": "block",
                "tag": "block"
            }
        ],
        "route": {
            "rules": route_rules,
            "auto_detect_interface": false
        }
    });

    Ok(config)
}

fn build_outbound(node: &ServerNode, routing_mark: u32) -> Result<Value> {
    if let Some(ref raw) = node.raw_link {
        match node.protocol {
            NodeProtocol::VLESS => {
                if let Ok(cfg) = parse_vless_uri(raw) {
                    let mut tls_obj = json!({
                        "enabled": cfg.security != "none",
                        "server_name": cfg.sni.unwrap_or_else(|| cfg.host.clone()),
                    });

                    if cfg.security == "reality" {
                        tls_obj["reality"] = json!({
                            "enabled": true,
                            "public_key": cfg.pbk.unwrap_or_default(),
                            "short_id": cfg.sid.unwrap_or_default()
                        });
                        if let Some(fp) = cfg.fp {
                            tls_obj["utls"] = json!({
                                "enabled": true,
                                "fingerprint": fp
                            });
                        }
                    }

                    let mut out = json!({
                        "type": "vless",
                        "tag": "proxy",
                        "server": cfg.host,
                        "server_port": cfg.port,
                        "uuid": cfg.uuid,
                        "tls": tls_obj,
                        "routing_mark": routing_mark
                    });

                    if let Some(flow) = cfg.flow {
                        out["flow"] = json!(flow);
                    }

                    if cfg.transport == "ws" {
                        out["transport"] = json!({
                            "type": "ws",
                            "path": cfg.path.unwrap_or_else(|| "/".to_string())
                        });
                    } else if cfg.transport == "grpc" {
                        out["transport"] = json!({
                            "type": "grpc",
                            "service_name": cfg.service_name.unwrap_or_default()
                        });
                    }

                    return Ok(out);
                }
            }
            NodeProtocol::Trojan => {
                if let Ok(cfg) = parse_trojan_uri(raw) {
                    return Ok(json!({
                        "type": "trojan",
                        "tag": "proxy",
                        "server": cfg.host,
                        "server_port": cfg.port,
                        "password": cfg.password,
                        "tls": {
                            "enabled": true,
                            "server_name": cfg.sni.unwrap_or_else(|| cfg.host.clone())
                        },
                        "routing_mark": routing_mark
                    }));
                }
            }
            NodeProtocol::VMess => {
                if let Ok(cfg) = parse_vmess_uri(raw) {
                    return Ok(json!({
                        "type": "vmess",
                        "tag": "proxy",
                        "server": cfg.host,
                        "server_port": cfg.port,
                        "uuid": cfg.uuid,
                        "security": "auto",
                        "tls": {
                            "enabled": cfg.security == "tls",
                            "server_name": cfg.sni.unwrap_or_else(|| cfg.host.clone())
                        },
                        "routing_mark": routing_mark
                    }));
                }
            }
            NodeProtocol::Shadowsocks => {
                if let Ok(cfg) = parse_ss_uri(raw) {
                    return Ok(json!({
                        "type": "shadowsocks",
                        "tag": "proxy",
                        "server": cfg.host,
                        "server_port": cfg.port,
                        "method": cfg.method,
                        "password": cfg.password,
                        "routing_mark": routing_mark
                    }));
                }
            }
            _ => {}
        }
    }

    // Fail closed: never downgrade encrypted protocols to an unencrypted SOCKS outbound
    Err(CoreError::Config(format!(
        "Cannot generate encrypted outbound for {:?} node '{}': missing or invalid raw profile URI",
        node.protocol, node.name
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_singbox_vless_reality() {
        let uri = "vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@195.181.162.88:443?type=tcp&security=reality&flow=xtls-rprx-vision&pbk=Z3b7K_sample&sid=1a2b&fp=chrome&sni=example.com#SE-01";
        let cfg = parse_vless_uri(uri).unwrap();
        let node = cfg.to_server_node(uri);

        let options = SingBoxConfigOptions::default();
        let config = generate_singbox_config(&node, &options).unwrap();

        assert_eq!(config["inbounds"][0]["listen_port"], 10808);
        assert_eq!(config["outbounds"][0]["type"], "vless");
        assert_eq!(config["outbounds"][0]["routing_mark"], 18256);
        assert_eq!(config["outbounds"][0]["tls"]["reality"]["enabled"], true);
        let rules = config["route"]["rules"].as_array().unwrap();
        assert!(rules.len() >= 4);
        assert!(rules
            .iter()
            .any(|r| r.get("geosite").map(|g| g[0] == "ir").unwrap_or(false)));
        assert_eq!(
            config["outbounds"][0]["tls"]["reality"]["public_key"],
            "Z3b7K_sample"
        );
    }

    #[test]
    fn test_empty_rules_does_not_inject_ir_bypass() {
        let uri = "vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@195.181.162.88:443?type=tcp&security=reality&pbk=Z3b7K_sample#SE-01";
        let node = parse_vless_uri(uri).unwrap().to_server_node(uri);

        let options = SingBoxConfigOptions {
            rules: Vec::new(),
            tun_interface: Some("ghost0".to_string()),
            ..Default::default()
        };
        let config = generate_singbox_config(&node, &options).unwrap();
        let rules = config["route"]["rules"].as_array().unwrap();
        assert!(
            !rules
                .iter()
                .any(|r| r.get("geosite").is_some() || r.get("geoip").is_some()),
            "Empty rule list must never silently inject .ir bypass rules"
        );
        assert!(config["inbounds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["type"] == "tun" && i["interface_name"] == "ghost0"));
    }

    #[test]
    fn test_malformed_vless_fails_closed_no_socks_fallback() {
        let node = ServerNode {
            id: "bad-1".to_string(),
            name: "Bad VLESS".to_string(),
            country_code: "DE".to_string(),
            city: "Frankfurt".to_string(),
            country: "Germany".to_string(),
            protocol: NodeProtocol::VLESS,
            transport: "TCP".to_string(),
            security: "Reality".to_string(),
            host: "198.51.100.1".to_string(),
            port: 443,
            latency_ms: None,
            pinging: None,
            load_percent: 0,
            subscription_id: None,
            raw_link: Some("not-a-vless-uri".to_string()),
        };

        let err = generate_singbox_config(&node, &SingBoxConfigOptions::default()).unwrap_err();
        assert!(err
            .to_string()
            .contains("missing or invalid raw profile URI"));
    }
}
