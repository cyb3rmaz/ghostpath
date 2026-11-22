use crate::models::{ServerNode, Subscription};
use base64::prelude::*;

pub struct SubscriptionManager;

impl SubscriptionManager {
    /// Parses raw subscription content (plain text or base64 encoded) into individual link URIs.
    pub fn parse_subscription_links(raw_content: &str) -> Vec<String> {
        let trimmed = raw_content.trim();
        let decoded = if let Ok(bytes) = BASE64_STANDARD.decode(trimmed.replace(['\r', '\n'], "")) {
            String::from_utf8(bytes).unwrap_or_else(|_| trimmed.to_string())
        } else {
            trimmed.to_string()
        };

        decoded
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && (l.contains("://") || l.starts_with("ss://")))
            .collect()
    }

    /// Evaluates whether a subscription is due for an automated update.
    pub fn should_update(sub: &Subscription, current_timestamp_secs: u64) -> bool {
        if sub.auto_update_hours == 0 {
            return false;
        }

        let last_ts = sub.last_updated.parse::<u64>().unwrap_or(0);

        let interval_secs = (sub.auto_update_hours as u64) * 3600;
        current_timestamp_secs.saturating_sub(last_ts) >= interval_secs
    }

    /// Updates node association with a subscription feed.
    pub fn associate_nodes_with_subscription(nodes: &mut [ServerNode], sub_id: &str) {
        for node in nodes.iter_mut() {
            node.subscription_id = Some(sub_id.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_subscription_links() {
        let content = "vless://user@host1:443?security=reality#Node1\nvmess://eyJ2IjoiMiJ9#Node2";
        let links = SubscriptionManager::parse_subscription_links(content);
        assert_eq!(links.len(), 2);
        assert!(links[0].starts_with("vless://"));
        assert!(links[1].starts_with("vmess://"));
    }

    #[test]
    fn test_parse_base64_subscription_links() {
        let raw = "vless://user@host1:443?security=reality#Node1\ntrojan://pass@host2:443#Node2";
        let encoded = BASE64_STANDARD.encode(raw);
        let links = SubscriptionManager::parse_subscription_links(&encoded);
        assert_eq!(links.len(), 2);
        assert!(links[0].starts_with("vless://"));
        assert!(links[1].starts_with("trojan://"));
    }

    #[test]
    fn test_subscription_should_update() {
        let sub = Subscription {
            id: "sub-1".to_string(),
            name: "Test Sub".to_string(),
            url: "https://example.com/feed".to_string(),
            used_gb: 10.0,
            total_gb: 100.0,
            expires_at: "2027-01-01".to_string(),
            last_updated: "1000".to_string(),
            auto_update_hours: 1,
            updating: None,
        };

        // 3600 seconds later -> true
        assert!(SubscriptionManager::should_update(&sub, 4600));
        // 100 seconds later -> false
        assert!(!SubscriptionManager::should_update(&sub, 1100));
    }
}
