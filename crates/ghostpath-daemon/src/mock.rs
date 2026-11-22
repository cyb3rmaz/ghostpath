use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use ghostpath_core::models::{
    AppSettings, ConnectionStatus, DiagnosticLog, LogLevel, NodeProtocol, RoutingMode, ServerNode,
    Subscription,
};
use ghostpath_core::state::StateManager;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

pub struct MockDaemonServer {
    port: u16,
    state_manager: Arc<StateManager>,
    settings: Arc<RwLock<AppSettings>>,
    nodes: Arc<RwLock<Vec<ServerNode>>>,
    subscriptions: Arc<RwLock<Vec<Subscription>>>,
    logs: Arc<RwLock<Vec<DiagnosticLog>>>,
}

impl MockDaemonServer {
    pub fn new(port: u16) -> Self {
        let sample_nodes = vec![
            ServerNode {
                id: "node-se-01".to_string(),
                name: "Stockholm SE-01".to_string(),
                country_code: "SE".to_string(),
                city: "Stockholm".to_string(),
                country: "Sweden".to_string(),
                protocol: NodeProtocol::VLESS,
                transport: "TCP · Vision".to_string(),
                security: "Reality".to_string(),
                host: "195.181.162.88".to_string(),
                port: 443,
                latency_ms: Some(34),
                pinging: Some(false),
                load_percent: 18,
                subscription_id: Some("sub-1".to_string()),
                raw_link: Some("vless://sample...".to_string()),
            },
            ServerNode {
                id: "node-de-02".to_string(),
                name: "Frankfurt DE-02".to_string(),
                country_code: "DE".to_string(),
                city: "Frankfurt".to_string(),
                country: "Germany".to_string(),
                protocol: NodeProtocol::Trojan,
                transport: "gRPC".to_string(),
                security: "TLS".to_string(),
                host: "185.158.248.31".to_string(),
                port: 443,
                latency_ms: Some(42),
                pinging: Some(false),
                load_percent: 29,
                subscription_id: Some("sub-1".to_string()),
                raw_link: Some("trojan://sample...".to_string()),
            },
        ];

        let sample_subs = vec![Subscription {
            id: "sub-1".to_string(),
            name: "Baltic Edge Relay".to_string(),
            url: "https://sub.baltic-mesh.io/v2/profile/linux-x86_64?token=49ac72e1".to_string(),
            used_gb: 142.5,
            total_gb: 1000.0,
            expires_at: "2027-11-22".to_string(),
            last_updated: "10m ago".to_string(),
            auto_update_hours: 12,
            updating: Some(false),
        }];

        let sample_logs = vec![
            DiagnosticLog {
                id: "log-1".to_string(),
                timestamp: "01:30:12.104".to_string(),
                level: LogLevel::Info,
                component: "daemon/init".to_string(),
                message: "GhostPath VPN daemon started in mock backend mode".to_string(),
            },
            DiagnosticLog {
                id: "log-2".to_string(),
                timestamp: "01:30:12.110".to_string(),
                level: LogLevel::Info,
                component: "net/route".to_string(),
                message: "Fwmark 0x4750 bypass and table 5180 verified".to_string(),
            },
        ];

        Self {
            port,
            state_manager: Arc::new(StateManager::new()),
            settings: Arc::new(RwLock::new(AppSettings::default())),
            nodes: Arc::new(RwLock::new(sample_nodes)),
            subscriptions: Arc::new(RwLock::new(sample_subs)),
            logs: Arc::new(RwLock::new(sample_logs)),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = TcpListener::bind(addr).await?;
        tracing::info!(
            port = self.port,
            "GhostPath mock JSON-RPC server listening on http://127.0.0.1:{}",
            self.port
        );

        loop {
            let (mut socket, peer) = listener.accept().await?;
            tracing::debug!(peer = %peer, "New client connection to mock daemon");

            let sm = self.state_manager.clone();
            let settings = self.settings.clone();
            let nodes = self.nodes.clone();
            let subs = self.subscriptions.clone();
            let logs = self.logs.clone();

            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                while let Ok(n) = socket.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    let req_str = String::from_utf8_lossy(&buf[..n]);

                    // Handle HTTP POST request or raw JSON-RPC frame
                    let json_payload =
                        if req_str.starts_with("POST ") || req_str.starts_with("GET ") {
                            // Extract HTTP body if POST
                            if let Some(pos) = req_str.find("\r\n\r\n") {
                                &req_str[pos + 4..]
                            } else {
                                ""
                            }
                        } else {
                            &req_str
                        };

                    if json_payload.trim().is_empty() {
                        let http_res = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nOK";
                        let _ = socket.write_all(http_res.as_bytes()).await;
                        continue;
                    }

                    if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(json_payload) {
                        let res = handle_rpc(req, &sm, &settings, &nodes, &subs, &logs).await;
                        let res_json = serde_json::to_string(&res).unwrap_or_default();

                        let response = if req_str.starts_with("POST ") {
                            format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
                                res_json.len(),
                                res_json
                            )
                        } else {
                            format!("{}\n", res_json)
                        };

                        let _ = socket.write_all(response.as_bytes()).await;
                    }
                }
            });
        }
    }
}

async fn handle_rpc(
    req: JsonRpcRequest,
    sm: &Arc<StateManager>,
    settings: &Arc<RwLock<AppSettings>>,
    nodes: &Arc<RwLock<Vec<ServerNode>>>,
    subs: &Arc<RwLock<Vec<Subscription>>>,
    logs: &Arc<RwLock<Vec<DiagnosticLog>>>,
) -> JsonRpcResponse {
    let method = req.method.as_str();
    let id = req.id;

    match method {
        "getStatus" => {
            let status = sm.current_status().await;
            let current = sm.current_state().await;
            let active_node = current.active_node.or_else(|| {
                // Default first node
                tokio::task::block_in_place(|| {
                    // For mock
                    None
                })
            });

            let traffic = sm.traffic_stats().await;

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "status": status,
                    "activeNode": active_node,
                    "routingMode": RoutingMode::Rule,
                    "traffic": traffic,
                })),
                error: None,
            }
        }
        "connect" => {
            let node_id = req.params.get("nodeId").and_then(|v| v.as_str());
            let node_list = nodes.read().await;
            let target = if let Some(id) = node_id {
                node_list.iter().find(|n| n.id == id).cloned()
            } else {
                node_list.first().cloned()
            };

            sm.set_active_node(target.clone()).await;
            let _ = sm.transition_to(ConnectionStatus::Connecting).await;
            let sm_clone = sm.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(400)).await;
                let _ = sm_clone.transition_to(ConnectionStatus::Connected).await;
            });

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "status": "connecting",
                    "node": target
                })),
                error: None,
            }
        }
        "disconnect" => {
            let _ = sm.transition_to(ConnectionStatus::Disconnected).await;
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "status": "disconnected"
                })),
                error: None,
            }
        }
        "listNodes" => {
            let list = nodes.read().await.clone();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!(list)),
                error: None,
            }
        }
        "pingNode" => {
            let latency = 32 + (rand_val() % 15);
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({ "latencyMs": latency })),
                error: None,
            }
        }
        "pingAll" => {
            let list = nodes.read().await;
            let results: Vec<Value> = list
                .iter()
                .map(|n| {
                    json!({
                        "id": n.id,
                        "latencyMs": 28 + (rand_val() % 20)
                    })
                })
                .collect();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({ "results": results })),
                error: None,
            }
        }
        "listSubscriptions" => {
            let list = subs.read().await.clone();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!(list)),
                error: None,
            }
        }
        "getSettings" => {
            let current_settings = settings.read().await.clone();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!(current_settings)),
                error: None,
            }
        }
        "getLogs" => {
            let list = logs.read().await.clone();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!(list)),
                error: None,
            }
        }
        "runDoctor" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "checks": [
                    { "id": "tun", "name": "Kernel TUN Device", "passed": true, "fix": null },
                    { "id": "privs", "name": "Daemon Capabilities", "passed": true, "fix": null },
                    { "id": "resolved", "name": "systemd-resolved D-Bus API", "passed": true, "fix": null },
                    { "id": "rp_filter", "name": "Reverse Path Filtering", "passed": true, "fix": null },
                    { "id": "docker", "name": "Docker Network Coexistence", "passed": true, "fix": null }
                ]
            })),
            error: None,
        },
        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(json!({
                "code": -32601,
                "message": format!("Method not found: {method}")
            })),
        },
    }
}

fn rand_val() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos()
}
