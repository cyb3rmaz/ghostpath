use crate::models::{RoutingRule, RuleAction, RuleType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEngine {
    rules: Vec<RoutingRule>,
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new(default_routing_rules())
    }
}

impl RuleEngine {
    pub fn new(rules: Vec<RoutingRule>) -> Self {
        Self { rules }
    }

    pub fn rules(&self) -> &[RoutingRule] {
        &self.rules
    }

    pub fn add_rule(&mut self, rule: RoutingRule) {
        self.rules.push(rule);
    }

    pub fn remove_rule(&mut self, id: &str) -> bool {
        let initial_len = self.rules.len();
        self.rules.retain(|r| r.id != id);
        self.rules.len() < initial_len
    }

    pub fn evaluate_domain(&self, domain: &str) -> RuleAction {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            match rule.r#type {
                RuleType::Domain => {
                    if domain == rule.pattern || domain.ends_with(&format!(".{}", rule.pattern)) {
                        return rule.action;
                    }
                }
                RuleType::Geosite => {
                    if rule.pattern == "ir" && (domain.ends_with(".ir") || domain.contains(".ir."))
                    {
                        return rule.action;
                    }
                    if rule.pattern == "category-ads-all" && is_ad_domain(domain) {
                        return rule.action;
                    }
                }
                _ => {}
            }
        }
        RuleAction::Proxy
    }

    pub fn evaluate_ip(&self, ip_str: &str) -> RuleAction {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            match rule.r#type {
                RuleType::Cidr if is_ip_in_cidr(ip_str, &rule.pattern) => {
                    return rule.action;
                }
                RuleType::Geoip if rule.pattern == "private" && is_private_ip(ip_str) => {
                    return rule.action;
                }
                _ => {}
            }
        }
        RuleAction::Proxy
    }
}

pub fn default_routing_rules() -> Vec<RoutingRule> {
    vec![
        RoutingRule {
            id: "rule-lan".to_string(),
            r#type: RuleType::Geoip,
            pattern: "private".to_string(),
            action: RuleAction::Direct,
            enabled: true,
            note: "Bypass private LAN subnets".to_string(),
        },
        RoutingRule {
            id: "rule-ir-domains".to_string(),
            r#type: RuleType::Geosite,
            pattern: "ir".to_string(),
            action: RuleAction::Direct,
            enabled: true,
            note: "Bypass Iranian domains (.ir)".to_string(),
        },
        RoutingRule {
            id: "rule-ir-ips".to_string(),
            r#type: RuleType::Geoip,
            pattern: "ir".to_string(),
            action: RuleAction::Direct,
            enabled: true,
            note: "Bypass Iranian IP addresses (geoip:ir)".to_string(),
        },
        RoutingRule {
            id: "rule-ads".to_string(),
            r#type: RuleType::Geosite,
            pattern: "category-ads-all".to_string(),
            action: RuleAction::Block,
            enabled: true,
            note: "Block common telemetry and ad networks".to_string(),
        },
    ]
}

fn is_ad_domain(domain: &str) -> bool {
    let ad_keywords = [
        "doubleclick.net",
        "google-analytics.com",
        "adservice",
        "telemetry",
    ];
    ad_keywords.iter().any(|&k| domain.contains(k))
}

fn is_private_ip(ip: &str) -> bool {
    if ip.starts_with("127.")
        || ip.starts_with("10.")
        || ip.starts_with("192.168.")
        || ip == "::1"
        || ip.starts_with("fe80:")
    {
        return true;
    }
    if let Some(rest) = ip.strip_prefix("172.") {
        if let Some(second_octet) = rest.split('.').next().and_then(|s| s.parse::<u8>().ok()) {
            return (16..=31).contains(&second_octet);
        }
    }
    false
}

fn is_ip_in_cidr(ip_str: &str, cidr: &str) -> bool {
    if let (Ok(ip), Ok(net)) = (ip_str.parse::<std::net::IpAddr>(), parse_cidr(cidr)) {
        net.contains(ip)
    } else {
        false
    }
}

struct IpNet {
    addr: u32,
    mask: u32,
}

impl IpNet {
    fn contains(&self, ip: std::net::IpAddr) -> bool {
        match ip {
            std::net::IpAddr::V4(v4) => {
                let u = u32::from(v4);
                (u & self.mask) == (self.addr & self.mask)
            }
            std::net::IpAddr::V6(_) => false,
        }
    }
}

fn parse_cidr(cidr: &str) -> Result<IpNet, ()> {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return Err(());
    }
    let ip = parts[0].parse::<std::net::Ipv4Addr>().map_err(|_| ())?;
    let prefix = parts[1].parse::<u32>().map_err(|_| ())?;
    if prefix > 32 {
        return Err(());
    }
    let mask = if prefix == 0 {
        0
    } else {
        !0u32 << (32 - prefix)
    };
    Ok(IpNet {
        addr: u32::from(ip),
        mask,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_rules_evaluation() {
        let engine = RuleEngine::default();
        // Iranian domain should be direct
        assert_eq!(
            engine.evaluate_domain("varzesh3.com.ir"),
            RuleAction::Direct
        );
        assert_eq!(engine.evaluate_domain("sanjesh.org.ir"), RuleAction::Direct);
        // Regular blocked site should be proxy
        assert_eq!(engine.evaluate_domain("youtube.com"), RuleAction::Proxy);
        assert_eq!(engine.evaluate_domain("twitter.com"), RuleAction::Proxy);
        // Ads should be blocked
        assert_eq!(
            engine.evaluate_domain("adservice.google.com"),
            RuleAction::Block
        );
        // Private IP should be direct
        assert_eq!(engine.evaluate_ip("192.168.1.1"), RuleAction::Direct);
        assert_eq!(engine.evaluate_ip("10.0.0.5"), RuleAction::Direct);
        assert_eq!(engine.evaluate_ip("1.1.1.1"), RuleAction::Proxy);
    }
}
