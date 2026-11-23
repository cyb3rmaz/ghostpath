use super::client::IpcClient;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tracing::debug;

/// Background task that queries daemon state and traffic metrics,
/// pushing updates to the Tauri frontend webview via Tauri events.
pub fn spawn_event_loop(app: AppHandle, client: IpcClient) {
    tauri::async_runtime::spawn(async move {
        debug!("Starting daemon event bridge loop");
        let mut last_status = String::new();

        loop {
            tokio::time::sleep(Duration::from_millis(1000)).await;

            if !client.is_available().await {
                if !last_status.is_empty() && last_status != "disconnected" {
                    last_status = "disconnected".to_string();
                    let _ = app.emit("state_changed", serde_json::json!({
                        "status": "disconnected",
                        "reason": "Daemon unreachable"
                    }));
                }
                continue;
            }

            match client.call("getStatus", serde_json::json!({})).await {
                Ok(status_val) => {
                    let current_status = status_val
                        .get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("disconnected")
                        .to_string();

                    if current_status != last_status {
                        last_status = current_status.clone();
                        let _ = app.emit("state_changed", status_val.clone());
                    }

                    if let Some(traffic) = status_val.get("traffic") {
                        let _ = app.emit("traffic_updated", traffic.clone());
                    }
                }
                Err(e) => {
                    debug!("Failed to poll status from daemon: {}", e);
                }
            }
        }
    });
}
