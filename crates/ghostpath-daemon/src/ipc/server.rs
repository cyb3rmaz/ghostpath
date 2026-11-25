use super::auth::verify_peer_credentials;
use super::handler::handle_rpc_request;
use super::protocol::{JsonRpcRequest, JsonRpcResponse, ERR_INVALID_REQUEST, ERR_PARSE};
use crate::engine::DaemonEngine;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, error, info, warn};

const MAX_IPC_LINE_BYTES: usize = 1024 * 1024; // 1 MiB
const MAX_CONCURRENT_IPC_CLIENTS: usize = 32;
const IPC_IDLE_READ_TIMEOUT: Duration = Duration::from_secs(30);
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(10);
const RATE_LIMIT_MAX_EXPENSIVE_RPCS: usize = 10;

#[derive(Default)]
struct UidRateLimiter {
    buckets: Mutex<HashMap<u32, Vec<Instant>>>,
}

impl UidRateLimiter {
    async fn check_and_record(&self, uid: u32, method: &str) -> bool {
        let is_expensive = matches!(
            method,
            "importSubscription" | "refreshSubscription" | "pingAll" | "connect" | "applyGeoSpoof"
        );
        if !is_expensive {
            return true;
        }
        let now = Instant::now();
        let mut guard = self.buckets.lock().await;
        let entries = guard.entry(uid).or_default();
        entries.retain(|t| now.duration_since(*t) < RATE_LIMIT_WINDOW);
        if entries.len() >= RATE_LIMIT_MAX_EXPENSIVE_RPCS {
            return false;
        }
        entries.push(now);
        true
    }
}

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
            if let Ok(meta) = std::fs::symlink_metadata(parent) {
                if meta.file_type().is_symlink() {
                    anyhow::bail!(
                        "Refusing symlinked IPC socket directory: {}",
                        parent.display()
                    );
                }
            } else {
                std::fs::create_dir_all(parent)?;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o755));
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

            // Attempt to assign group ownership to 'ghostpath' if it exists
            if let Ok(Some(group)) = nix::unistd::Group::from_name("ghostpath") {
                if let Err(e) = nix::unistd::chown(&self.socket_path, None, Some(group.gid)) {
                    debug!("Could not chown socket to ghostpath group: {}", e);
                }
            }
        }

        info!(
            socket = %self.socket_path.display(),
            "GhostPath daemon IPC server listening"
        );

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_IPC_CLIENTS));
        let rate_limiter = Arc::new(UidRateLimiter::default());

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let Ok(permit) = semaphore.clone().try_acquire_owned() else {
                        warn!("Rejecting IPC connection: max concurrent client limit ({MAX_CONCURRENT_IPC_CLIENTS}) reached");
                        continue;
                    };
                    let engine = self.engine.clone();
                    let rl = rate_limiter.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        if let Err(e) = handle_client(stream, engine, rl).await {
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

async fn handle_client(
    stream: tokio::net::UnixStream,
    engine: Arc<DaemonEngine>,
    rate_limiter: Arc<UidRateLimiter>,
) -> Result<()> {
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

    // 2. Read line-by-line JSON-RPC requests (bounded to MAX_IPC_LINE_BYTES)
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut buf = Vec::with_capacity(4096);

    loop {
        buf.clear();
        let mut hit_eof = false;
        let mut too_large = false;
        loop {
            let fill_res = tokio::time::timeout(IPC_IDLE_READ_TIMEOUT, buf_reader.fill_buf()).await;
            let available = match fill_res {
                Ok(res) => res?,
                Err(_) => {
                    warn!(
                        uid = peer.uid,
                        "Closing idle IPC client connection after timeout"
                    );
                    return Ok(());
                }
            };
            if available.is_empty() {
                hit_eof = true;
                break;
            }
            if let Some(pos) = available.iter().position(|&b| b == b'\n') {
                let take = pos + 1;
                if buf.len() + take > MAX_IPC_LINE_BYTES {
                    too_large = true;
                    buf_reader.consume(take);
                    break;
                }
                buf.extend_from_slice(&available[..take]);
                buf_reader.consume(take);
                break;
            } else {
                let len = available.len();
                if buf.len() + len > MAX_IPC_LINE_BYTES {
                    too_large = true;
                    buf_reader.consume(len);
                    break;
                }
                buf.extend_from_slice(available);
                buf_reader.consume(len);
            }
        }

        if too_large {
            let err_resp = JsonRpcResponse::error(
                None,
                ERR_INVALID_REQUEST,
                format!("Request exceeds maximum size of {MAX_IPC_LINE_BYTES} bytes"),
            );
            let mut out_str = serde_json::to_string(&err_resp)?;
            out_str.push('\n');
            writer.write_all(out_str.as_bytes()).await?;
            writer.flush().await?;
            break;
        }

        if hit_eof && buf.is_empty() {
            break;
        }

        let line = String::from_utf8_lossy(&buf);
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
                } else if !rate_limiter.check_and_record(peer.uid, &req.method).await {
                    warn!(uid = peer.uid, method = %req.method, "IPC rate limit exceeded");
                    JsonRpcResponse::error(
                        req.id,
                        ERR_INVALID_REQUEST,
                        "Rate limit exceeded for privileged operation; please wait",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ipc_rate_limiter_bounds_expensive_rpcs() {
        let rl = UidRateLimiter::default();
        for _ in 0..RATE_LIMIT_MAX_EXPENSIVE_RPCS {
            assert!(rl.check_and_record(1000, "importSubscription").await);
        }
        assert!(
            !rl.check_and_record(1000, "importSubscription").await,
            "11th expensive RPC within window must be rate-limited"
        );
        // Cheap read-only RPCs remain allowed
        assert!(rl.check_and_record(1000, "getStatus").await);
    }
}
