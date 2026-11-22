use super::auth::verify_peer_credentials;
use super::handler::handle_rpc_request;
use super::protocol::{JsonRpcRequest, JsonRpcResponse, ERR_INVALID_REQUEST, ERR_PARSE};
use crate::engine::DaemonEngine;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tracing::{debug, error, info, warn};

pub struct IpcServer {
    socket_path: PathBuf,
    engine: Arc<DaemonEngine>,
}

impl IpcServer {
    pub fn new(socket_path: impl Into<PathBuf>, engine: Arc<DaemonEngine>) -> Self {
        Self {
            socket_path: socket_path.into(),
            engine,
        }
    }

    pub async fn run(&self) -> Result<()> {
        if let Some(parent) = self.socket_path.parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }

        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }

        let listener = UnixListener::bind(&self.socket_path)
            .with_context(|| format!("Failed to bind Unix socket at {:?}", self.socket_path))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o660);
            if let Err(e) = std::fs::set_permissions(&self.socket_path, perms) {
                warn!("Could not set socket permissions to 0660: {}", e);
            }
        }

        info!(
            socket = %self.socket_path.display(),
            "GhostPath daemon IPC server listening"
        );

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let engine = self.engine.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, engine).await {
                            debug!("IPC client session ended: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting IPC connection: {}", e);
                }
            }
        }
    }

    pub fn cleanup(&self) {
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
            info!(socket = %self.socket_path.display(), "IPC socket removed");
        }
    }
}

async fn handle_client(stream: tokio::net::UnixStream, engine: Arc<DaemonEngine>) -> Result<()> {
    // 1. Verify peer credentials
    let peer = match verify_peer_credentials(&stream) {
        Ok(p) => p,
        Err(e) => {
            warn!("Rejected unauthorized connection: {}", e);
            let mut writer = stream;
            let err_resp = JsonRpcResponse::error(None, -32000, format!("Access Denied: {}", e));
            let mut msg = serde_json::to_string(&err_resp)?;
            msg.push('\n');
            let _ = writer.write_all(msg.as_bytes()).await;
            return Ok(());
        }
    };

    debug!(uid = peer.uid, "Accepted authorized IPC client connection");

    // 2. Read line-by-line JSON-RPC requests
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = buf_reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            // EOF
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let resp = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => {
                if req.jsonrpc != "2.0" {
                    JsonRpcResponse::error(
                        req.id,
                        ERR_INVALID_REQUEST,
                        "Invalid JSON-RPC version (must be '2.0')",
                    )
                } else {
                    handle_rpc_request(engine.clone(), req).await
                }
            }
            Err(e) => JsonRpcResponse::error(
                None,
                ERR_PARSE,
                format!("Failed to parse JSON-RPC request: {}", e),
            ),
        };

        let mut out_str = serde_json::to_string(&resp)?;
        out_str.push('\n');
        writer.write_all(out_str.as_bytes()).await?;
        writer.flush().await?;
    }

    Ok(())
}
