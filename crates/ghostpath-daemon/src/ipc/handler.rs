use super::protocol::*;
use crate::engine::DaemonEngine;
use std::sync::Arc;
use tracing::{debug, info};

pub async fn handle_rpc_request(engine: Arc<DaemonEngine>, req: JsonRpcRequest) -> JsonRpcResponse {
    let id = req.id.clone();
    debug!(method = %req.method, "Handling JSON-RPC request");

    match req.method.as_str() {
        "connect" => {
            let node_id = req
                .params
                .as_ref()
                .and_then(|p| p.get("nodeId"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            match engine.connect(node_id).await {
                Ok(res) => JsonRpcResponse::success(id, res),
                Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
            }
        }
        "disconnect" => match engine.disconnect().await {
            Ok(res) => JsonRpcResponse::success(id, res),
            Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
        },
        "getStatus" => match engine.get_status().await {
            Ok(res) => JsonRpcResponse::success(id, res),
            Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL, e.to_string()),
        },
        "importProfile" => {
            let raw = req
                .params
                .as_ref()
                .and_then(|p| p.get("raw"))
                .and_then(|v| v.as_str());
            let name = req
                .params
                .as_ref()
                .and_then(|p| p.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            match raw {
                Some(r) => match engine.import_profile(r, name).await {
                    Ok(res) => JsonRpcResponse::success(id, res),
                    Err(e) => JsonRpcResponse::error(id, ERR_INVALID_PARAMS, e.to_string()),
                },
                None => {
                    JsonRpcResponse::error(id, ERR_INVALID_PARAMS, "Missing 'raw' URI in params")
                }
            }
        }
        "listNodes" => match engine.list_nodes().await {
            Ok(res) => JsonRpcResponse::success(id, res),
            Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL, e.to_string()),
        },
        "pingNode" => {
            let node_id = req
                .params
                .as_ref()
                .and_then(|p| p.get("nodeId"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match engine.ping_node(node_id).await {
                Ok(res) => JsonRpcResponse::success(id, res),
                Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
            }
        }
        "pingAll" => match engine.ping_all().await {
            Ok(res) => JsonRpcResponse::success(id, res),
            Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
        },
        "listSubscriptions" => match engine.list_subscriptions().await {
            Ok(res) => JsonRpcResponse::success(id, res),
            Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL, e.to_string()),
        },
        "importSubscription" => {
            let url = req
                .params
                .as_ref()
                .and_then(|p| p.get("url"))
                .and_then(|v| v.as_str());
            let name = req
                .params
                .as_ref()
                .and_then(|p| p.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            match url {
                Some(u) => match engine.import_subscription(u, name).await {
                    Ok(res) => JsonRpcResponse::success(id, res),
                    Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
                },
                None => JsonRpcResponse::error(id, ERR_INVALID_PARAMS, "Missing 'url' parameter"),
            }
        }
        "deleteSubscription" => {
            let sub_id = req
                .params
                .as_ref()
                .and_then(|p| p.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match engine.delete_subscription(sub_id).await {
                Ok(res) => JsonRpcResponse::success(id, res),
                Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
            }
        }
        "refreshSubscription" => {
            let sub_id = req
                .params
                .as_ref()
                .and_then(|p| p.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match engine.refresh_subscription(sub_id).await {
                Ok(res) => JsonRpcResponse::success(id, res),
                Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
            }
        }
        "listRules" => match engine.list_rules().await {
            Ok(res) => JsonRpcResponse::success(id, res),
            Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL, e.to_string()),
        },
        "addRule" => {
            let rule = req.params.unwrap_or(serde_json::Value::Null);
            match engine.add_rule(rule).await {
                Ok(res) => JsonRpcResponse::success(id, res),
                Err(e) => JsonRpcResponse::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }
        "deleteRule" => {
            let rule_id = req
                .params
                .as_ref()
                .and_then(|p| p.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match engine.delete_rule(rule_id).await {
                Ok(res) => JsonRpcResponse::success(id, res),
                Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
            }
        }
        "toggleRule" => {
            let rule_id = req
                .params
                .as_ref()
                .and_then(|p| p.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match engine.toggle_rule(rule_id).await {
                Ok(res) => JsonRpcResponse::success(id, res),
                Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
            }
        }
        "updateRuleAction" => {
            let rule_id = req
                .params
                .as_ref()
                .and_then(|p| p.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let action_val = req
                .params
                .as_ref()
                .and_then(|p| p.get("action"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            match serde_json::from_value::<ghostpath_core::models::RuleAction>(action_val) {
                Ok(action) => match engine.update_rule_action(rule_id, action).await {
                    Ok(res) => JsonRpcResponse::success(id, res),
                    Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
                },
                Err(e) => JsonRpcResponse::error(
                    id,
                    ERR_INVALID_PARAMS,
                    format!("Invalid rule action: {e}"),
                ),
            }
        }
        "listGeoCountries" => match engine.list_geo_countries().await {
            Ok(res) => JsonRpcResponse::success(id, res),
            Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL, e.to_string()),
        },
        "getGeoSpoofStatus" => match engine.get_geo_spoof_status().await {
            Ok(res) => JsonRpcResponse::success(id, res),
            Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL, e.to_string()),
        },
        "updateGeoSpoof" => {
            let patch = req.params.unwrap_or(serde_json::Value::Null);
            match engine.update_geo_spoof(patch).await {
                Ok(res) => JsonRpcResponse::success(id, res),
                Err(e) => JsonRpcResponse::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }
        "applyGeoSpoof" => {
            let country_code = req
                .params
                .as_ref()
                .and_then(|p| p.get("countryCode"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            match engine.apply_geo_spoof(country_code).await {
                Ok(res) => JsonRpcResponse::success(id, res),
                Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
            }
        }
        "restoreGeoSpoof" => match engine.restore_geo_spoof().await {
            Ok(res) => JsonRpcResponse::success(id, res),
            Err(e) => JsonRpcResponse::error(id, ERR_SERVER, e.to_string()),
        },
        "getSettings" => match engine.get_settings().await {
            Ok(res) => JsonRpcResponse::success(id, res),
            Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL, e.to_string()),
        },
        "updateSettings" => {
            let patch = req.params.unwrap_or(serde_json::Value::Null);
            match engine.update_settings(patch).await {
                Ok(res) => JsonRpcResponse::success(id, res),
                Err(e) => JsonRpcResponse::error(id, ERR_INVALID_PARAMS, e.to_string()),
            }
        }
        "getLogs" => {
            let limit = req
                .params
                .as_ref()
                .and_then(|p| p.get("limit"))
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as usize;
            match engine.get_logs(limit).await {
                Ok(res) => JsonRpcResponse::success(id, res),
                Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL, e.to_string()),
            }
        }
        "runDoctor" => match engine.run_doctor().await {
            Ok(res) => JsonRpcResponse::success(id, res),
            Err(e) => JsonRpcResponse::error(id, ERR_INTERNAL, e.to_string()),
        },
        unknown => {
            info!(method = unknown, "Received unknown JSON-RPC method");
            JsonRpcResponse::error(
                id,
                ERR_METHOD_NOT_FOUND,
                format!("Method '{}' not found", unknown),
            )
        }
    }
}
