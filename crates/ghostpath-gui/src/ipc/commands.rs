use super::client::IpcClient;
use super::daemon_status::{check_daemon_health, DaemonHealth};
use super::mock::MockDaemon;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, warn};

pub struct AppState {
    pub ipc_client: IpcClient,
    pub mock_daemon: Arc<MockDaemon>,
    pub force_mock: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let force_mock = std::env::var("GHOSTPATH_MOCK_GUI").map(|v| v == "1").unwrap_or(false);
        Self {
            ipc_client: IpcClient::default(),
            mock_daemon: Arc::new(MockDaemon::new()),
            force_mock,
        }
    }
}

impl AppState {
    pub fn new(ipc_client: IpcClient, force_mock: bool) -> Self {
        Self {
            ipc_client,
            mock_daemon: Arc::new(MockDaemon::new()),
            force_mock,
        }
    }
}

pub async fn handle_rpc_invoke(
    state: &AppState,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    debug!(method = %method, "Processing RPC invoke");

    if !state.force_mock && state.ipc_client.is_available().await {
        match state.ipc_client.call(method, params.clone()).await {
            Ok(res) => return Ok(res),
            Err(e) => {
                warn!("Live daemon IPC call [{}] failed: {}. Falling back to mock/offline", method, e);
            }
        }
    }

    // Offline / Mock fallback
    state.mock_daemon.handle_rpc(method, &params)
}

pub async fn handle_check_daemon_status(state: &AppState) -> DaemonHealth {
    if state.force_mock {
        return DaemonHealth::running(env!("CARGO_PKG_VERSION"));
    }
    check_daemon_health(state.ipc_client.socket_path(), env!("CARGO_PKG_VERSION")).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::daemon_status::DaemonStateKind;
    use serde_json::json;

    #[tokio::test]
    async fn test_rpc_invoke_mock_mode() {
        let state = AppState {
            ipc_client: IpcClient::new("/non_existent.sock"),
            mock_daemon: Arc::new(MockDaemon::new()),
            force_mock: true,
        };

        let res = handle_rpc_invoke(&state, "getStatus", json!({})).await.unwrap();
        assert_eq!(res["status"], "disconnected");
        assert_eq!(res["interface"], "ghost0");
    }

    #[tokio::test]
    async fn test_check_daemon_status_mock() {
        let state = AppState {
            ipc_client: IpcClient::new("/non_existent.sock"),
            mock_daemon: Arc::new(MockDaemon::new()),
            force_mock: true,
        };

        let health = handle_check_daemon_status(&state).await;
        assert_eq!(health.state, DaemonStateKind::Running);
    }
}
