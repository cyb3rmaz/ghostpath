use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{CoreError, Result};
use crate::models::{ConnectionStatus, ServerNode, TrafficStats};

#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub status: ConnectionStatus,
    pub active_node: Option<ServerNode>,
    pub connected_at: Option<u64>,
    pub error_message: Option<String>,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            status: ConnectionStatus::Disconnected,
            active_node: None,
            connected_at: None,
            error_message: None,
        }
    }
}

pub struct StateManager {
    state: Arc<RwLock<ConnectionState>>,
    total_download_bytes: Arc<AtomicU64>,
    total_upload_bytes: Arc<AtomicU64>,
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(ConnectionState::default())),
            total_download_bytes: Arc::new(AtomicU64::new(0)),
            total_upload_bytes: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn current_status(&self) -> ConnectionStatus {
        self.state.read().await.status
    }

    pub async fn current_state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }

    pub async fn transition_to(&self, new_status: ConnectionStatus) -> Result<()> {
        let mut state = self.state.write().await;
        let current = state.status;

        let valid = match (current, new_status) {
            (ConnectionStatus::Disconnected, ConnectionStatus::Connecting) => true,
            (ConnectionStatus::Connecting, ConnectionStatus::Connected) => true,
            (ConnectionStatus::Connecting, ConnectionStatus::Failed) => true,
            (ConnectionStatus::Connecting, ConnectionStatus::Disconnected) => true,
            (ConnectionStatus::Connected, ConnectionStatus::Degraded) => true,
            (ConnectionStatus::Connected, ConnectionStatus::Disconnecting) => true,
            (ConnectionStatus::Connected, ConnectionStatus::Disconnected) => true,
            (ConnectionStatus::Connected, ConnectionStatus::Failed) => true,
            (ConnectionStatus::Degraded, ConnectionStatus::Connected) => true,
            (ConnectionStatus::Degraded, ConnectionStatus::Disconnecting) => true,
            (ConnectionStatus::Degraded, ConnectionStatus::Failed) => true,
            (ConnectionStatus::Degraded, ConnectionStatus::Disconnected) => true,
            (ConnectionStatus::Disconnecting, ConnectionStatus::Disconnected) => true,
            (ConnectionStatus::Failed, ConnectionStatus::Disconnected) => true,
            (ConnectionStatus::Failed, ConnectionStatus::Connecting) => true,
            // Idempotent transitions
            (a, b) if a == b => true,
            _ => false,
        };

        if !valid {
            return Err(CoreError::InvalidStateTransition {
                from: format!("{current:?}"),
                to: format!("{new_status:?}"),
                reason: "Transition disallowed by state machine".to_string(),
            });
        }

        state.status = new_status;
        if new_status == ConnectionStatus::Connected {
            state.connected_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
            state.error_message = None;
        } else if new_status == ConnectionStatus::Disconnected {
            state.connected_at = None;
        }

        Ok(())
    }

    pub async fn set_active_node(&self, node: Option<ServerNode>) {
        let mut state = self.state.write().await;
        state.active_node = node;
    }

    pub async fn set_error(&self, err: String) {
        let mut state = self.state.write().await;
        state.error_message = Some(err);
        state.status = ConnectionStatus::Failed;
    }

    pub fn record_traffic(&self, down_bytes: u64, up_bytes: u64) {
        self.total_download_bytes
            .fetch_add(down_bytes, Ordering::Relaxed);
        self.total_upload_bytes
            .fetch_add(up_bytes, Ordering::Relaxed);
    }

    pub async fn traffic_stats(&self) -> TrafficStats {
        let state = self.state.read().await;
        let session_seconds = if let Some(connected_at) = state.connected_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            now.saturating_sub(connected_at)
        } else {
            0
        };

        let down_b = self.total_download_bytes.load(Ordering::Relaxed);
        let up_b = self.total_upload_bytes.load(Ordering::Relaxed);

        TrafficStats {
            download_mbps: 0.0,
            upload_mbps: 0.0,
            total_down_mb: (down_b as f64) / (1024.0 * 1024.0),
            total_up_mb: (up_b as f64) / (1024.0 * 1024.0),
            session_seconds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_transitions() {
        let sm = StateManager::new();
        assert_eq!(sm.current_status().await, ConnectionStatus::Disconnected);

        sm.transition_to(ConnectionStatus::Connecting)
            .await
            .unwrap();
        assert_eq!(sm.current_status().await, ConnectionStatus::Connecting);

        sm.transition_to(ConnectionStatus::Connected).await.unwrap();
        assert_eq!(sm.current_status().await, ConnectionStatus::Connected);

        sm.transition_to(ConnectionStatus::Disconnecting)
            .await
            .unwrap();
        sm.transition_to(ConnectionStatus::Disconnected)
            .await
            .unwrap();

        // Disallowed: Disconnected -> Connected directly without Connecting
        let res = sm.transition_to(ConnectionStatus::Connected).await;
        assert!(res.is_err());
    }
}
