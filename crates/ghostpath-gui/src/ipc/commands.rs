use super::client::IpcClient;
use super::daemon_status::{check_daemon_health, DaemonHealth};
#[cfg(any(test, feature = "dev-mock"))]
use super::mock::MockDaemon;
use serde_json::Value;
#[cfg(any(test, feature = "dev-mock"))]
use std::sync::Arc;
use tracing::{debug, warn};

pub struct AppState {
    pub ipc_client: IpcClient,
    #[cfg(any(test, feature = "dev-mock"))]
    pub mock_daemon: Arc<MockDaemon>,
    #[cfg(any(test, feature = "dev-mock"))]
    pub force_mock: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for AppState {
    fn default() -> Self {
        #[cfg(any(test, feature = "dev-mock"))]
        {
            let force_mock = std::env::var("GHOSTPATH_MOCK_GUI")
                .map(|v| v == "1")
                .unwrap_or(false);
            Self {
                ipc_client: IpcClient::default(),
                mock_daemon: Arc::new(MockDaemon::new()),
                force_mock,
            }
        }
        #[cfg(not(any(test, feature = "dev-mock")))]
        {
            Self {
                ipc_client: IpcClient::default(),
            }
        }
    }
}

impl AppState {
    #[cfg(any(test, feature = "dev-mock"))]
    pub fn new(ipc_client: IpcClient, force_mock: bool) -> Self {
        Self {
            ipc_client,
            mock_daemon: Arc::new(MockDaemon::new()),
            force_mock,
        }
    }

    #[cfg(not(any(test, feature = "dev-mock")))]
    pub fn new(ipc_client: IpcClient) -> Self {
        Self { ipc_client }
    }
}

pub async fn handle_rpc_invoke(
    state: &AppState,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    debug!(method = %method, "Processing RPC invoke");

    #[cfg(any(test, feature = "dev-mock"))]
    {
        if state.force_mock {
            return state.mock_daemon.handle_rpc(method, &params);
        }
    }

    if state.ipc_client.is_available().await {
        match state.ipc_client.call(method, params.clone()).await {
            Ok(res) => return Ok(res),
            Err(e) => {
                warn!(
                    "Live daemon IPC call [{}] failed: {}",
                    method, e
                );
                return Err(format!("Daemon IPC call '{method}' failed: {e}"));
            }
        }
    }

    #[cfg(any(test, feature = "dev-mock"))]
    {
        warn!("Daemon IPC unreachable; falling back to mock");
        state.mock_daemon.handle_rpc(method, &params)
    }

    #[cfg(not(any(test, feature = "dev-mock")))]
    {
        Err("GhostPath daemon is not running. Please start the ghostpathd service.".to_string())
    }
}

pub async fn handle_check_daemon_status(state: &AppState) -> DaemonHealth {
    #[cfg(any(test, feature = "dev-mock"))]
    {
        if state.force_mock {
            return DaemonHealth::running(env!("CARGO_PKG_VERSION"));
        }
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

        let res = handle_rpc_invoke(&state, "getStatus", json!({}))
            .await
            .unwrap();
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
