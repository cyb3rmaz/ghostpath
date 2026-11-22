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
