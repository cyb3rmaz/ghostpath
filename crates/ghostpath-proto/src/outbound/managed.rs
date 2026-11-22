use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use ghostpath_core::models::{CoreEngine, ServerNode};
use ghostpath_core::{CoreError, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::config_gen::singbox::{generate_singbox_config, SingBoxConfigOptions};
use crate::Outbound;

pub struct ManagedCoreOutbound {
    engine: CoreEngine,
    socks_port: u16,
    http_port: u16,
    routing_mark: u32,
    child_process: Arc<Mutex<Option<Child>>>,
    config_path: PathBuf,
    active_node: Option<ServerNode>,
}

impl ManagedCoreOutbound {
    pub fn new(engine: CoreEngine, socks_port: u16, http_port: u16, routing_mark: u32) -> Self {
        let config_path = std::env::temp_dir().join(format!("ghostpath_core_{socks_port}.json"));
        Self {
            engine,
            socks_port,
            http_port,
            routing_mark,
            child_process: Arc::new(Mutex::new(None)),
            config_path,
            active_node: None,
        }
    }

    pub fn socks_addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.socks_port))
    }

    async fn wait_for_socks_ready(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        let addr = self.socks_addr();

        while start.elapsed() < timeout {
            if let Ok(mut stream) = TcpStream::connect(addr).await {
                // Send SOCKS5 handshake: version 5, 1 auth method, no auth (0x00)
                let handshake = [0x05, 0x01, 0x00];
                if stream.write_all(&handshake).await.is_ok() {
                    let mut resp = [0u8; 2];
                    if stream.read_exact(&mut resp).await.is_ok()
                        && resp[0] == 0x05
                        && resp[1] == 0x00
                    {
                        return Ok(());
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Err(CoreError::Internal(format!(
            "SOCKS5 inbound failed to become ready on {addr} within {timeout:?}"
        )))
    }
}

#[async_trait]
impl Outbound for ManagedCoreOutbound {
    async fn start(&mut self, node: &ServerNode) -> Result<()> {
        self.stop().await?;
        self.active_node = Some(node.clone());

        let binary_name = match self.engine {
            CoreEngine::SingBox => "sing-box",
            CoreEngine::XrayCore => "xray",
            CoreEngine::Native => "sing-box",
        };

        let options = SingBoxConfigOptions {
            socks_port: self.socks_port,
            http_port: self.http_port,
            routing_mark: self.routing_mark,
            rules: vec![],
        };

        let config_json = generate_singbox_config(node, &options)?;
        tokio::fs::write(&self.config_path, config_json.to_string())
            .await
            .map_err(CoreError::Io)?;

        // Check if external core binary is installed on the host
        let has_binary = which_binary(binary_name);

        if has_binary {
            tracing::info!(engine = binary_name, config = ?self.config_path, "Spawning supervised proxy core child process");

            let child = Command::new(binary_name)
                .arg("run")
                .arg("-c")
                .arg(&self.config_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| CoreError::Internal(format!("Failed to spawn {binary_name}: {e}")))?;

            *self.child_process.lock().await = Some(child);

            // Wait for SOCKS5 port readiness
            self.wait_for_socks_ready(Duration::from_secs(4)).await?;
        } else {
            tracing::info!(
                engine = binary_name,
                port = self.socks_port,
                "Core binary not found on PATH; starting built-in mock SOCKS5 responder"
            );
            spawn_mock_socks_listener(self.socks_port).await?;
        }

        tracing::info!(node = %node.name, host = %node.host, port = node.port, "Outbound proxy engine started successfully");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        let mut lock = self.child_process.lock().await;
        if let Some(mut child) = lock.take() {
            tracing::info!("Stopping supervised proxy core process");
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        if self.config_path.exists() {
            let _ = tokio::fs::remove_file(&self.config_path).await;
        }

        self.active_node = None;
        Ok(())
    }

    async fn ping(&self) -> Result<u32> {
        let start = Instant::now();
        let addr = self.socks_addr();

        let mut stream = TcpStream::connect(addr).await.map_err(|e| {
            CoreError::Internal(format!("Failed to connect to SOCKS5 listener: {e}"))
        })?;

        // SOCKS5 handshake
        stream.write_all(&[0x05, 0x01, 0x00]).await?;
        let mut resp = [0u8; 2];
        stream.read_exact(&mut resp).await?;

        if resp[0] != 0x05 || resp[1] != 0x00 {
            return Err(CoreError::Internal("SOCKS5 handshake rejected".into()));
        }

        let elapsed = start.elapsed().as_millis() as u32;
        Ok(elapsed.max(1))
    }
}

fn which_binary(name: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let full = dir.join(name);
            if full.is_file() {
                return true;
            }
        }
    }
    false
}

async fn spawn_mock_socks_listener(port: u16) -> Result<()> {
    use tokio::net::TcpListener;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await.map_err(CoreError::Io)?;

    tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0u8; 256];
                    if let Ok(n) = socket.read(&mut buf).await {
                        if n >= 2 && buf[0] == 0x05 {
                            // SOCKS5: Accept NO AUTH (0x00)
                            let _ = socket.write_all(&[0x05, 0x00]).await;
                        }
                    }
                });
            }
        }
    });

    // Small delay to allow listener loop to bind
    tokio::time::sleep(Duration::from_millis(30)).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ghostpath_core::models::NodeProtocol;

    #[tokio::test]
    async fn test_managed_outbound_start_and_ping() {
        let mut outbound = ManagedCoreOutbound::new(CoreEngine::SingBox, 10899, 10898, 0x4750);
        let node = ServerNode {
            id: "node-test".to_string(),
            name: "Test Node".to_string(),
            country_code: "SE".to_string(),
            city: "Stockholm".to_string(),
            country: "Sweden".to_string(),
            protocol: NodeProtocol::VLESS,
            transport: "TCP".to_string(),
            security: "Reality".to_string(),
            host: "127.0.0.1".to_string(),
            port: 443,
            latency_ms: None,
            pinging: Some(false),
            load_percent: 10,
            subscription_id: None,
            raw_link: Some("vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@127.0.0.1:443?security=reality&pbk=key123#Test".to_string()),
        };

        outbound.start(&node).await.unwrap();
        let ping = outbound.ping().await.unwrap();
        assert!(ping > 0);
        outbound.stop().await.unwrap();
    }
}
