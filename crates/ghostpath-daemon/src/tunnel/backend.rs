use async_trait::async_trait;
use ghostpath_core::error::Result;
use ghostpath_net::journal::UndoJournal;
use ghostpath_proto::amnezia::AwgProfileConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TunnelStats {
    pub interface_name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub last_handshake_secs: Option<u64>,
    pub endpoint: String,
}

#[async_trait]
pub trait TunnelBackend: Send + Sync {
    /// Starts the tunnel interface, configures keys and obfuscation parameters,
    /// sets addresses, and brings up the interface.
    async fn start(
        &mut self,
        config: &AwgProfileConfig,
        dry_run: bool,
        journal: &mut UndoJournal,
    ) -> Result<()>;

    /// Stops the tunnel interface and cleans up resources.
    async fn stop(&mut self, dry_run: bool, journal: &mut UndoJournal) -> Result<()>;

    /// Returns active real-time stats (bytes, handshake, latency).
    async fn stats(&self) -> Result<TunnelStats>;

    /// Returns true if the tunnel interface is active.
    async fn is_running(&self) -> bool;
}
