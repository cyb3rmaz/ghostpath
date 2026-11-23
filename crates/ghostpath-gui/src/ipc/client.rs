use anyhow::Result;
#[cfg(unix)]
use anyhow::Context;
use serde_json::Value;
#[cfg(unix)]
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
#[cfg(unix)]
use std::sync::atomic::Ordering;
#[cfg(unix)]
use std::time::Duration;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixStream;
#[cfg(unix)]
use tracing::debug;

pub const DEFAULT_SOCKET_PATH: &str = "/run/ghostpath/ghostpathd.sock";

#[derive(Debug, Clone)]
pub struct IpcClient {
    socket_path: PathBuf,
    req_id: std::sync::Arc<AtomicU64>,
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new(get_default_socket_path())
    }
}

pub fn get_default_socket_path() -> PathBuf {
    std::env::var("GHOSTPATH_SOCKET")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET_PATH))
}

impl IpcClient {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: path.into(),
            req_id: std::sync::Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Check if the Unix domain socket is active and reachable.
    pub async fn is_available(&self) -> bool {
        #[cfg(unix)]
        {
            if !self.socket_path.exists() {
                return false;
            }
            tokio::time::timeout(
                Duration::from_millis(200),
                UnixStream::connect(&self.socket_path),
            )
            .await
            .map(|r| r.is_ok())
            .unwrap_or(false)
        }
        #[cfg(not(unix))]
        {
            false
        }
    }

    /// Send a JSON-RPC 2.0 request over the Unix domain socket and await the response.
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        #[cfg(unix)]
        {
            let id = self.req_id.fetch_add(1, Ordering::SeqCst);
            let payload = json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            });

            debug!(method = %method, id = id, socket = %self.socket_path.display(), "Dispatching IPC request");

            let connect_fut = UnixStream::connect(&self.socket_path);
            let mut stream = tokio::time::timeout(Duration::from_millis(800), connect_fut)
                .await
                .map_err(|_| {
                    anyhow::anyhow!(
                        "Timeout connecting to GhostPath daemon at {:?}",
                        self.socket_path
                    )
                })?
                .with_context(|| {
                    format!(
                        "Failed to connect to GhostPath daemon at {:?}",
                        self.socket_path
                    )
                })?;

            let mut out_str = serde_json::to_string(&payload)?;
            out_str.push('\n');

            stream.write_all(out_str.as_bytes()).await?;
            stream.flush().await?;

            let (reader, _) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut response_line = String::new();

            let read_fut = buf_reader.read_line(&mut response_line);
            tokio::time::timeout(Duration::from_secs(5), read_fut)
                .await
                .map_err(|_| anyhow::anyhow!("Timeout waiting for response from GhostPath daemon"))?
                .context("Failed to read response from GhostPath daemon")?;

            let resp_json: Value = serde_json::from_str(response_line.trim())
                .with_context(|| format!("Malformed JSON-RPC response: {:?}", response_line))?;

            if let Some(err) = resp_json.get("error") {
                let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                let message = err
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown daemon error");
                anyhow::bail!("Daemon error [{}]: {}", code, message);
            }

            resp_json.get("result").cloned().ok_or_else(|| {
                anyhow::anyhow!("Missing 'result' in daemon response: {:?}", resp_json)
            })
        }
        #[cfg(not(unix))]
        {
            let _ = (method, params);
            anyhow::bail!("Unix domain sockets are unsupported on this platform");
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::net::UnixListener;

    #[tokio::test]
    async fn test_ipc_client_roundtrip() {
        let dir = tempdir().unwrap();
        let sock_path = dir.path().join("test_daemon.sock");

        let listener = UnixListener::bind(&sock_path).unwrap();

        // Spawn mock server
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();

            buf_reader.read_line(&mut line).await.unwrap();
            let req: Value = serde_json::from_str(line.trim()).unwrap();
            assert_eq!(req["method"], "getStatus");

            let resp = json!({
                "jsonrpc": "2.0",
                "id": req["id"],
                "result": {
                    "status": "connected",
                    "interface": "ghost0",
                    "traffic": {
                        "downloadMbps": 45.2,
                        "uploadMbps": 8.7
                    }
                }
            });
            let mut resp_str = serde_json::to_string(&resp).unwrap();
            resp_str.push('\n');
            writer.write_all(resp_str.as_bytes()).await.unwrap();
        });

        let client = IpcClient::new(&sock_path);
        let res = client.call("getStatus", json!({})).await.unwrap();
        assert_eq!(res["status"], "connected");
        assert_eq!(res["interface"], "ghost0");
        assert_eq!(res["traffic"]["downloadMbps"], 45.2);
    }

    #[tokio::test]
    async fn test_ipc_client_error_response() {
        let dir = tempdir().unwrap();
        let sock_path = dir.path().join("test_err_daemon.sock");

        let listener = UnixListener::bind(&sock_path).unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (reader, mut writer) = stream.into_split();
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();

            buf_reader.read_line(&mut line).await.unwrap();
            let req: Value = serde_json::from_str(line.trim()).unwrap();

            let resp = json!({
                "jsonrpc": "2.0",
                "id": req["id"],
                "error": {
                    "code": -32601,
                    "message": "Method 'unknownMethod' not found"
                }
            });
            let mut resp_str = serde_json::to_string(&resp).unwrap();
            resp_str.push('\n');
            writer.write_all(resp_str.as_bytes()).await.unwrap();
        });

        let client = IpcClient::new(&sock_path);
        let err = client.call("unknownMethod", json!({})).await.unwrap_err();
        assert!(err.to_string().contains("Daemon error [-32601]"));
    }

    #[tokio::test]
    async fn test_ipc_client_not_running() {
        let dir = tempdir().unwrap();
        let sock_path = dir.path().join("non_existent.sock");

        let client = IpcClient::new(&sock_path);
        assert!(!client.is_available().await);
        let err = client.call("getStatus", json!({})).await.unwrap_err();
        assert!(
            err.to_string().contains("Failed to connect") || err.to_string().contains("Timeout")
        );
    }
}
