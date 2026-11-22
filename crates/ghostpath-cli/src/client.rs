use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Serialize, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcResponse {
    jsonrpc: String,
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<RpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
    data: Option<serde_json::Value>,
}

pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    pub fn default_socket_path() -> PathBuf {
        std::env::var("GHOSTPATH_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/run/ghostpath/ghostpathd.sock"))
    }

    pub async fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let stream = match UnixStream::connect(&self.socket_path).await {
            Ok(s) => s,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    bail!(
                        "Cannot connect to GhostPath daemon at '{}': Permission denied.\n\
                         Please ensure your user is in the 'ghostpath', 'sudo', or 'wheel' group:\n  \
                         sudo usermod -aG ghostpath $USER",
                        self.socket_path.display()
                    );
                } else {
                    bail!(
                        "Cannot connect to GhostPath daemon at '{}': {}\n\
                         Please ensure the daemon is running:\n  \
                         sudo systemctl start ghostpathd\n\
                         Or run in foreground:\n  \
                         sudo ghostpathd",
                        self.socket_path.display(),
                        e
                    );
                }
            }
        };

        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        let req = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params,
        };

        let mut payload = serde_json::to_string(&req)?;
        payload.push('\n');
        writer.write_all(payload.as_bytes()).await?;
        writer.flush().await?;

        let mut line = String::new();
        buf_reader
            .read_line(&mut line)
            .await
            .context("Daemon closed connection unexpectedly")?;

        let resp: RpcResponse = serde_json::from_str(&line)
            .with_context(|| format!("Invalid JSON response from daemon: {}", line))?;

        if let Some(err) = resp.error {
            bail!("Daemon error [{}]: {}", err.code, err.message);
        }

        resp.result
            .context("Daemon returned empty result without error")
    }
}
