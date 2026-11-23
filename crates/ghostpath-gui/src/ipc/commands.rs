use super::mock::MockDaemon;
use serde_json::Value;
use std::sync::Arc;
use tracing::debug;

pub struct AppState {
    pub mock_daemon: Arc<MockDaemon>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mock_daemon: Arc::new(MockDaemon::new()),
        }
    }
}

pub async fn handle_rpc_invoke(
    mock: &MockDaemon,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    debug!(method = %method, "Processing RPC invoke");
    mock.handle_rpc(method, &params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_rpc_invoke_mock() {
        let mock = MockDaemon::new();
        let res = handle_rpc_invoke(&mock, "getStatus", json!({})).await.unwrap();
        assert_eq!(res["status"], "disconnected");
    }
}
