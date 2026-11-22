pub mod auth;
pub mod handler;
pub mod protocol;
pub mod server;

pub use auth::{verify_peer_credentials, PeerInfo};
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use server::IpcServer;
