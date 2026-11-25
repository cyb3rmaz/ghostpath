use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use ghostpath_core::models::{CoreEngine, RoutingRule, ServerNode};
use ghostpath_core::{
    atomic_write_private, validate_iface_name, validate_local_port, validate_mtu,
    validate_tun_stack, CoreError, GhostPaths, Result,
};
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
    rules: Vec<RoutingRule>,
    tun_interface: Option<String>,
    tun_mtu: u16,
    tun_stack: String,
    child_process: Arc<Mutex<Option<Child>>>,
    config_path: PathBuf,
    active_node: Option<ServerNode>,
}

impl Drop for ManagedCoreOutbound {
    fn drop(&mut self) {
        if self.config_path.exists() {
            let _ = std::fs::remove_file(&self.config_path);
        }
    }
}

impl ManagedCoreOutbound {
    pub fn new(engine: CoreEngine, socks_port: u16, http_port: u16, routing_mark: u32) -> Self {
        let nonce = uuid::Uuid::new_v4().simple();
        let config_path =
            GhostPaths::runtime_core_dir().join(format!("core_{socks_port}_{nonce}.json"));
        Self {
            engine,
            socks_port,
            http_port,
            routing_mark,
            rules: Vec::new(),
            tun_interface: None,
            tun_mtu: 1420,
            tun_stack: "system".to_string(),
            child_process: Arc::new(Mutex::new(None)),
            config_path,
            active_node: None,
        }
    }

    pub fn with_rules(mut self, rules: Vec<RoutingRule>) -> Self {
        self.rules = rules;
        self
    }

    pub fn with_tun(
        mut self,
        iface: impl Into<String>,
        mtu: u16,
        stack: impl Into<String>,
    ) -> Self {
        self.tun_interface = Some(iface.into());
        self.tun_mtu = mtu;
        self.tun_stack = stack.into();
        self
    }

    pub fn socks_addr(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.socks_port))
    }

    /// Starts outbound with explicit mock permission (`false` in production so missing binaries fail closed).
    pub async fn start_with_mock_policy(
        &mut self,
        node: &ServerNode,
        allow_mock: bool,
    ) -> Result<()> {
        self.stop().await?;
        validate_local_port(self.socks_port)?;
        validate_local_port(self.http_port)?;
        validate_mtu(self.tun_mtu)?;
        validate_tun_stack(&self.tun_stack)?;
        if let Some(ref iface) = self.tun_interface {
            validate_iface_name(iface)?;
        }

        self.active_node = Some(node.clone());

        let binary_name = match self.engine {
            CoreEngine::SingBox | CoreEngine::Native => "sing-box",
            CoreEngine::XrayCore => "xray",
        };

        let options = SingBoxConfigOptions {
            socks_port: self.socks_port,
            http_port: self.http_port,
            routing_mark: self.routing_mark,
            rules: self.rules.clone(),
            tun_interface: self.tun_interface.clone(),
            tun_mtu: self.tun_mtu,
            tun_stack: self.tun_stack.clone(),
        };

        let config_json = generate_singbox_config(node, &options)?;
        atomic_write_private(&self.config_path, config_json.to_string().as_bytes())?;

        if let Some(binary_path) = resolve_trusted_binary(binary_name) {
            tracing::info!(
                engine = binary_name,
                "Spawning supervised proxy core child process"
            );

            let child = Command::new(binary_path)
                .arg("run")
                .arg("-c")
                .arg(&self.config_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| CoreError::Internal(format!("Failed to spawn {binary_name}: {e}")))?;

            *self.child_process.lock().await = Some(child);

            if let Err(err) = self.wait_for_socks_ready(Duration::from_secs(4)).await {
                let _ = self.stop().await;
                return Err(err);
            }
        } else if allow_mock {
            #[cfg(any(test, feature = "dev-mock"))]
            {
                tracing::warn!(
                    engine = binary_name,
                    port = self.socks_port,
                    "dev-mock enabled: core binary absent, starting test SOCKS5 responder"
                );
                spawn_mock_socks_listener(self.socks_port).await?;
            }
            #[cfg(not(any(test, feature = "dev-mock")))]
            {
                let _ = self.stop().await;
                return Err(CoreError::Internal(format!(
                    "Required proxy core binary '{binary_name}' is not installed on trusted system paths"
                )));
            }
        } else {
            let _ = self.stop().await;
            return Err(CoreError::Internal(format!(
                "Required proxy core binary '{binary_name}' is not installed on trusted system paths"
            )));
        }

        tracing::info!(node = %node.name, host = %node.host, port = node.port, "Outbound proxy engine started successfully");
        Ok(())
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
        self.start_with_mock_policy(node, cfg!(any(test, feature = "dev-mock")))
            .await
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

fn resolve_trusted_binary(name: &str) -> Option<PathBuf> {
    if name.contains('/') || name.contains('\\') || name.starts_with('-') {
        return None;
    }
    const TRUSTED_DIRS: &[&str] = &["/usr/bin", "/usr/local/bin", "/usr/sbin", "/bin"];
    for dir in TRUSTED_DIRS {
        let candidate = PathBuf::from(dir).join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(any(test, feature = "dev-mock"))]
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
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("GHOSTPATH_RUNTIME_DIR", dir.path());

        let mut outbound = ManagedCoreOutbound::new(CoreEngine::SingBox, 10899, 10898, 0x4750);
        assert!(!outbound
            .config_path
            .starts_with(std::env::temp_dir().join("ghostpath_core_10899.json")));
        let node = ServerNode {
            id: "node-test".to_string(),
            name: "Test Node".to_string(),
            country_code: "SE".to_string(),
            city: "Stockholm".to_string(),
            country: "Sweden".to_string(),
            protocol: NodeProtocol::VLESS,
            transport: "TCP".to_string(),
            security: "Reality".to_string(),
            host: "198.51.100.10".to_string(),
            port: 443,
            latency_ms: None,
            pinging: Some(false),
            load_percent: 10,
            subscription_id: None,
            raw_link: Some("vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@198.51.100.10:443?security=reality&pbk=key123#Test".to_string()),
        };

        outbound.start(&node).await.unwrap();
        assert!(outbound.config_path.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&outbound.config_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
        let ping = outbound.ping().await.unwrap();
        assert!(ping > 0);
        outbound.stop().await.unwrap();
        assert!(!outbound.config_path.exists());
    }

    #[tokio::test]
    async fn test_missing_binary_fails_closed_without_mock() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("GHOSTPATH_RUNTIME_DIR", dir.path());

        let mut outbound = ManagedCoreOutbound::new(CoreEngine::XrayCore, 10901, 10902, 0x4750);
        let node = ServerNode {
            id: "node-fail".to_string(),
            name: "Fail Node".to_string(),
            country_code: "DE".to_string(),
            city: "Frankfurt".to_string(),
            country: "Germany".to_string(),
            protocol: NodeProtocol::VLESS,
            transport: "TCP".to_string(),
            security: "Reality".to_string(),
            host: "198.51.100.11".to_string(),
            port: 443,
            latency_ms: None,
            pinging: Some(false),
            load_percent: 10,
            subscription_id: None,
            raw_link: Some("vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@198.51.100.11:443?security=reality&pbk=key123#Fail".to_string()),
        };

        // Unless xray happens to be installed on host in /usr/bin, allow_mock = false must reject fail-open
        if resolve_trusted_binary("xray").is_none() {
            let err = outbound
                .start_with_mock_policy(&node, false)
                .await
                .unwrap_err();
            assert!(err.to_string().contains("not installed"));
            assert!(!outbound.config_path.exists());
        }
    }
}
