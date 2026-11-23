pub mod amnezia;
pub mod config_gen;
pub mod outbound;
pub mod parser;

use async_trait::async_trait;
use ghostpath_core::models::ServerNode;
use ghostpath_core::Result;

pub use config_gen::singbox::{generate_singbox_config, SingBoxConfigOptions};
pub use outbound::managed::ManagedCoreOutbound;
pub use parser::parse_any_link;

#[async_trait]
pub trait Outbound: Send + Sync {
    /// Starts the outbound connection or core engine for the specified node
    async fn start(&mut self, node: &ServerNode) -> Result<()>;

    /// Stops the outbound engine
    async fn stop(&mut self) -> Result<()>;

    /// Health check / latency test to server
    async fn ping(&self) -> Result<u32>;
}
