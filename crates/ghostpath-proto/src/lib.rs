use async_trait::async_trait;
use ghostpath_core::{Result, ServerNode};

#[async_trait]
pub trait Outbound: Send + Sync {
    /// Starts the outbound connection or core engine for the specified node
    async fn start(&mut self, node: &ServerNode) -> Result<()>;

    /// Stops the outbound engine
    async fn stop(&mut self) -> Result<()>;

    /// Health check / latency test to server
    async fn ping(&self) -> Result<u32>;
}
