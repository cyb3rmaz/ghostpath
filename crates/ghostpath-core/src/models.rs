use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Degraded,
    Failed,
    Disconnecting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RoutingMode {
    #[default]
    Rule,
    Global,
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkInterfaceMode {
    #[default]
    Tun,
    SystemProxy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NodeProtocol {
    #[default]
    VLESS,
    Hysteria2,
    WireGuard,
    Trojan,
    VMess,
    Shadowsocks,
    AmneziaWG,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum CoreEngine {
    #[default]
    SingBox,
    XrayCore,
    Native,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerNode {
    pub id: String,
    pub name: String,
    pub country_code: String,
    pub city: String,
    pub country: String,
    pub protocol: NodeProtocol,
    pub transport: String,
    pub security: String,
    pub host: String,
    pub port: u16,
    pub latency_ms: Option<u32>,
    pub pinging: Option<bool>,
    pub load_percent: u8,
    pub subscription_id: Option<String>,
    #[serde(default)]
    pub raw_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub url: String,
    pub used_gb: f64,
    pub total_gb: f64,
    pub expires_at: String,
    pub last_updated: String,
    pub auto_update_hours: u32,
    pub updating: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    #[default]
    Proxy,
    Direct,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuleType {
    #[default]
    Domain,
    Geosite,
    Geoip,
    Cidr,
    Process,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingRule {
    pub id: String,
    pub r#type: RuleType,
    pub pattern: String,
    pub action: RuleAction,
    pub enabled: bool,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    #[default]
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticLog {
    pub id: String,
    pub timestamp: String,
    pub level: LogLevel,
    pub component: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficPoint {
    pub time: String,
    pub download_mbps: f64,
    pub upload_mbps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficStats {
    pub download_mbps: f64,
    pub upload_mbps: f64,
    pub total_down_mb: f64,
    pub total_up_mb: f64,
    pub session_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub core_engine: CoreEngine,
    pub launch_on_boot: bool,
    pub kill_switch: bool,
    pub ipv6_routing: bool,
    pub allow_lan: bool,
    pub tun_stack: String,
    pub mtu: u16,
    pub socks_port: u16,
    pub http_port: u16,
    pub dns_primary: String,
    pub dns_fallback: String,
    #[serde(default)]
    pub excluded_interfaces: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            core_engine: CoreEngine::SingBox,
            launch_on_boot: false,
            kill_switch: true,
            ipv6_routing: false,
            allow_lan: true,
            tun_stack: "system".to_string(),
            mtu: 1420,
            socks_port: 10808,
            http_port: 10809,
            dns_primary: "1.1.1.1".to_string(),
            dns_fallback: "8.8.8.8".to_string(),
            excluded_interfaces: vec![
                "docker0".to_string(),
                "br-*".to_string(),
                "virbr*".to_string(),
            ],
        }
    }
}
