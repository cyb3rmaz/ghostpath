#![cfg(any(test, feature = "dev-mock"))]

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

/// In-memory mock daemon for offline testing and development of ghostpath-gui.
#[derive(Debug, Clone)]
pub struct MockDaemon {
    state: Arc<Mutex<MockState>>,
}

#[derive(Debug, Clone)]
struct MockState {
    status: String,
    active_node_id: Option<String>,
    settings: Value,
    nodes: Vec<Value>,
    logs: Vec<Value>,
}

impl Default for MockDaemon {
    fn default() -> Self {
        Self::new()
    }
}

impl MockDaemon {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState {
                status: "disconnected".to_string(),
                active_node_id: Some("mock-node-1".to_string()),
                settings: json!({
                    "coreEngine": "sing-box",
                    "launchOnBoot": false,
                    "killSwitch": true,
                    "ipv6Routing": true,
                    "allowLan": false,
                    "mtu": 1420,
                    "socksPort": 10808,
                    "httpPort": 10809,
                    "dnsPrimary": "1.1.1.1",
                    "dnsFallback": "8.8.8.8",
                    "excludedInterfaces": ["docker0", "wg0"]
                }),
                nodes: vec![
                    json!({
                        "id": "mock-node-1",
                        "name": "Frankfurt Reality DE-01",
                        "protocol": "VLESS",
                        "host": "195.181.162.88",
                        "port": 443,
                        "latencyMs": 32,
                        "region": "DE"
                    }),
                    json!({
                        "id": "mock-node-2",
                        "name": "Amsterdam Vision NL-02",
                        "protocol": "VLESS",
                        "host": "185.220.101.5",
                        "port": 443,
                        "latencyMs": 39,
                        "region": "NL"
                    }),
                ],
                logs: vec![json!({
                    "id": "1",
                    "timestamp": "20:00:01",
                    "level": "info",
                    "target": "ghostpath-daemon",
                    "message": "GhostPath mock daemon initialized"
                })],
            })),
        }
    }

    pub fn handle_rpc(&self, method: &str, params: &Value) -> Result<Value, String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        match method {
            "connect" => {
                let node_id = params
                    .get("nodeId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| state.active_node_id.clone())
                    .unwrap_or_else(|| "mock-node-1".to_string());

                state.status = "connected".to_string();
                state.active_node_id = Some(node_id.clone());
                Ok(json!({
                    "status": "connected",
                    "nodeId": node_id,
                    "interface": "ghost0"
                }))
            }
            "disconnect" => {
                state.status = "disconnected".to_string();
                Ok(json!({
                    "status": "disconnected"
                }))
            }
            "getStatus" => {
                let active_node = state.active_node_id.as_ref().and_then(|id| {
                    state
                        .nodes
                        .iter()
                        .find(|n| n.get("id").and_then(|v| v.as_str()) == Some(id))
                        .cloned()
                });

                Ok(json!({
                    "status": state.status,
                    "activeNode": active_node,
                    "routingMode": "rule",
                    "interfaceMode": "tun",
                    "traffic": {
                        "downloadMbps": if state.status == "connected" { 24.5 } else { 0.0 },
                        "uploadMbps": if state.status == "connected" { 5.2 } else { 0.0 },
                        "totalDownMb": if state.status == "connected" { 120.4 } else { 0.0 },
                        "totalUpMb": if state.status == "connected" { 28.1 } else { 0.0 },
                        "sessionSeconds": if state.status == "connected" { 180 } else { 0 }
                    },
                    "interface": "ghost0",
                    "mtu": 1420,
                    "core": "sing-box"
                }))
            }
            "listNodes" => Ok(Value::Array(state.nodes.clone())),
            "importProfile" => {
                let raw = params.get("raw").and_then(|v| v.as_str()).unwrap_or("");
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Imported Node");

                let new_node = json!({
                    "id": format!("node-{}", state.nodes.len() + 1),
                    "name": name,
                    "protocol": if raw.starts_with("vless://") { "VLESS" } else { "VMess" },
                    "host": "127.0.0.1",
                    "port": 443,
                    "latencyMs": 45,
                    "region": "CUSTOM"
                });
                state.nodes.push(new_node.clone());
                Ok(json!({
                    "type": "single_node",
                    "node": new_node
                }))
            }
            "getSettings" => Ok(state.settings.clone()),
            "updateSettings" => {
                if let Some(obj) = params.as_object() {
                    if let Some(target) = state.settings.as_object_mut() {
                        for (k, v) in obj {
                            target.insert(k.clone(), v.clone());
                        }
                    }
                }
                Ok(state.settings.clone())
            }
            "getLogs" => Ok(Value::Array(state.logs.clone())),
            "runDoctor" => Ok(json!({
                "checks": [
                    { "id": "tun", "name": "Kernel TUN Device", "passed": true, "fix": null },
                    { "id": "privs", "name": "Daemon Capabilities", "passed": true, "fix": null },
                    { "id": "dns", "name": "systemd-resolved D-Bus", "passed": true, "fix": null },
                    { "id": "rp_filter", "name": "Reverse Path Filtering", "passed": true, "fix": null },
                    { "id": "docker", "name": "Docker Bridge Interference", "passed": true, "fix": null }
                ],
                "overall_healthy": true
            })),
            unknown => Err(format!("Unknown RPC method: {}", unknown)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_daemon_lifecycle() {
        let daemon = MockDaemon::new();

        // 1. Initial status is disconnected
        let status = daemon.handle_rpc("getStatus", &json!({})).unwrap();
        assert_eq!(status["status"], "disconnected");
        assert_eq!(status["interface"], "ghost0");

        // 2. Connect
        let connect_res = daemon
            .handle_rpc("connect", &json!({"nodeId": "mock-node-1"}))
            .unwrap();
        assert_eq!(connect_res["status"], "connected");

        // 3. Status is now connected
        let status_after = daemon.handle_rpc("getStatus", &json!({})).unwrap();
        assert_eq!(status_after["status"], "connected");
        assert_eq!(status_after["traffic"]["downloadMbps"], 24.5);

        // 4. Disconnect
        let disc_res = daemon.handle_rpc("disconnect", &json!({})).unwrap();
        assert_eq!(disc_res["status"], "disconnected");
    }

    #[test]
    fn test_mock_daemon_import_and_nodes() {
        let daemon = MockDaemon::new();
        let nodes_before = daemon.handle_rpc("listNodes", &json!({})).unwrap();
        let count_before = nodes_before.as_array().unwrap().len();

        let imported = daemon
            .handle_rpc(
                "importProfile",
                &json!({
                    "raw": "vless://test@1.2.3.4:443",
                    "name": "Custom Frankfurt"
                }),
            )
            .unwrap();
        assert_eq!(imported["type"], "single_node");
        assert_eq!(imported["node"]["name"], "Custom Frankfurt");

        let nodes_after = daemon.handle_rpc("listNodes", &json!({})).unwrap();
        assert_eq!(nodes_after.as_array().unwrap().len(), count_before + 1);
    }
}
