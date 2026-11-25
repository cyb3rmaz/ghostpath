use super::backend::{TunnelBackend, TunnelStats};
use async_trait::async_trait;
use base64::prelude::*;
use ghostpath_core::error::{CoreError, Result};
use ghostpath_core::validators::{
    validate_cidr, validate_host_port, validate_iface_name, validate_uapi_value,
};
use ghostpath_net::journal::{JournalAction, UndoJournal};
use ghostpath_proto::amnezia::AwgProfileConfig;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tracing::{info, warn};

pub const DEFAULT_AWG_INTERFACE: &str = "awg-ghost0";
pub const DEFAULT_AWG_FWMARK: u32 = 0x4750;

pub struct AwgBackend {
    pub interface_name: String,
    pub fwmark: u32,
    child: Option<Child>,
    is_running: bool,
    last_config: Option<AwgProfileConfig>,
    cached_stats: TunnelStats,
}

impl Default for AwgBackend {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl AwgBackend {
    pub fn new(interface_name: Option<&str>, fwmark: Option<u32>) -> Self {
        let iface = interface_name.unwrap_or(DEFAULT_AWG_INTERFACE).to_string();
        Self {
            interface_name: iface.clone(),
            fwmark: fwmark.unwrap_or(DEFAULT_AWG_FWMARK),
            child: None,
            is_running: false,
            last_config: None,
            cached_stats: TunnelStats {
                interface_name: iface,
                rx_bytes: 0,
                tx_bytes: 0,
                last_handshake_secs: None,
                endpoint: String::new(),
            },
        }
    }

    /// Converts a 32-byte Base64 key string into a 64-character lowercase hexadecimal string
    /// as required by the WireGuard / AmneziaWG Userspace API (UAPI).
    pub fn base64_to_hex(b64: &str) -> Result<String> {
        let bytes = BASE64_STANDARD
            .decode(b64.trim())
            .or_else(|_| BASE64_URL_SAFE_NO_PAD.decode(b64.trim()))
            .map_err(|e| CoreError::Config(format!("Invalid Base64 key: {e}")))?;

        if bytes.len() != 32 {
            return Err(CoreError::Config(format!(
                "Key must be exactly 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut hex = String::with_capacity(64);
        for b in bytes {
            hex.push_str(&format!("{b:02x}"));
        }
        Ok(hex)
    }

    /// Generates standard WireGuard / AmneziaWG UAPI payload lines.
    pub fn build_uapi_config(&self, config: &AwgProfileConfig) -> Result<String> {
        validate_iface_name(&self.interface_name)?;
        let mut uapi = String::with_capacity(512);

        // Interface section
        uapi.push_str("set=1\n");
        let private_key_hex = Self::base64_to_hex(&config.interface.private_key)?;
        uapi.push_str(&format!("private_key={private_key_hex}\n"));

        // Obfuscation parameters (AmneziaWG extensions)
        let obf = &config.interface.obfuscation;
        if let Some(jc) = obf.jc {
            uapi.push_str(&format!("jc={jc}\n"));
        }
        if let Some(jmin) = obf.jmin {
            uapi.push_str(&format!("jmin={jmin}\n"));
        }
        if let Some(jmax) = obf.jmax {
            uapi.push_str(&format!("jmax={jmax}\n"));
        }
        if let Some(s1) = obf.s1 {
            uapi.push_str(&format!("s1={s1}\n"));
        }
        if let Some(s2) = obf.s2 {
            uapi.push_str(&format!("s2={s2}\n"));
        }
        if let Some(s3) = obf.s3 {
            uapi.push_str(&format!("s3={s3}\n"));
        }
        if let Some(s4) = obf.s4 {
            uapi.push_str(&format!("s4={s4}\n"));
        }
        if let Some(h1) = obf.h1 {
            uapi.push_str(&format!("h1={h1}\n"));
        }
        if let Some(h2) = obf.h2 {
            uapi.push_str(&format!("h2={h2}\n"));
        }
        if let Some(h3) = obf.h3 {
            uapi.push_str(&format!("h3={h3}\n"));
        }
        if let Some(h4) = obf.h4 {
            uapi.push_str(&format!("h4={h4}\n"));
        }
        for (key, val) in [
            ("i1", &obf.i1),
            ("i2", &obf.i2),
            ("i3", &obf.i3),
            ("i4", &obf.i4),
            ("i5", &obf.i5),
        ] {
            if let Some(ref v) = val {
                validate_uapi_value(v)?;
                uapi.push_str(&format!("{key}={v}\n"));
            }
        }

        // Peer section
        let public_key_hex = Self::base64_to_hex(&config.peer.public_key)?;
        uapi.push_str(&format!("public_key={public_key_hex}\n"));

        if let Some(ref psk) = config.peer.preshared_key {
            let psk_hex = Self::base64_to_hex(psk)?;
            uapi.push_str(&format!("preshared_key={psk_hex}\n"));
        }

        validate_uapi_value(&config.peer.endpoint)?;
        validate_host_port(&config.peer.endpoint)?;
        uapi.push_str(&format!("endpoint={}\n", config.peer.endpoint));

        for ip in &config.peer.allowed_ips {
            validate_uapi_value(ip)?;
            validate_cidr(ip)?;
            uapi.push_str(&format!("allowed_ip={ip}\n"));
        }

        if let Some(keepalive) = config.peer.persistent_keepalive {
            uapi.push_str(&format!("persistent_keepalive_interval={keepalive}\n"));
        }

        Ok(uapi)
    }

    /// Finds the `amneziawg-go` binary in trusted system locations only.
    pub fn find_binary() -> Option<PathBuf> {
        let candidates = [
            PathBuf::from("/usr/bin/amneziawg-go"),
            PathBuf::from("/usr/local/bin/amneziawg-go"),
            PathBuf::from("/var/lib/ghostpath/bin/amneziawg-go"),
        ];

        candidates.into_iter().find(|c| c.is_file())
    }

    /// Locates the UAPI Unix domain socket for the interface.
    pub fn uapi_socket_path(&self) -> PathBuf {
        let var_run = PathBuf::from(format!("/var/run/wireguard/{}.sock", self.interface_name));
        if var_run.exists() {
            return var_run;
        }
        PathBuf::from(format!("/run/wireguard/{}.sock", self.interface_name))
    }
}

#[async_trait]
impl TunnelBackend for AwgBackend {
    async fn start(
        &mut self,
        config: &AwgProfileConfig,
        dry_run: bool,
        journal: &mut UndoJournal,
    ) -> Result<()> {
        info!(
            "Starting AwgBackend on interface '{}' for profile '{}' (dry_run: {})",
            self.interface_name, config.name, dry_run
        );

        config
            .validate()
            .map_err(|e| CoreError::Config(e.to_string()))?;
        let uapi_payload = self.build_uapi_config(config)?;

        if dry_run {
            journal.record(JournalAction::CreatedInterface {
                name: self.interface_name.clone(),
            });
            self.is_running = true;
            self.last_config = Some(config.clone());
            self.cached_stats.endpoint = config.peer.endpoint.clone();
            return Ok(());
        }

        let binary_path = Self::find_binary().ok_or_else(|| {
            CoreError::Internal(
                "amneziawg-go binary not found on system. Please ensure ghostpath package is installed."
                    .to_string(),
            )
        })?;

        // 1. Spawn amneziawg-go with WG_PROCESS_MARK to prevent routing loops
        let mut cmd = Command::new(binary_path);
        cmd.arg(&self.interface_name);
        cmd.env("WG_PROCESS_MARK", format!("0x{:x}", self.fwmark));
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| CoreError::Internal(format!("Failed to spawn amneziawg-go: {e}")))?;

        // Wait up to 3 seconds for UAPI socket to appear
        let socket_path = self.uapi_socket_path();
        let mut socket_ready = false;
        for _ in 0..30 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            if socket_path.exists() {
                socket_ready = true;
                break;
            }
            if let Ok(Some(status)) = child.try_wait() {
                return Err(CoreError::Internal(format!(
                    "amneziawg-go exited prematurely with status {status}"
                )));
            }
        }

        if !socket_ready {
            let _ = child.kill().await;
            return Err(CoreError::Internal(format!(
                "UAPI socket at {:?} did not appear in time",
                socket_path
            )));
        }

        // 2. Connect to UAPI socket and send configuration lines
        let mut stream = UnixStream::connect(&socket_path)
            .await
            .map_err(|e| CoreError::Internal(format!("Failed to connect to UAPI socket: {e}")))?;

        stream
            .write_all(uapi_payload.as_bytes())
            .await
            .map_err(|e| CoreError::Internal(format!("Failed to write UAPI configuration: {e}")))?;

        // Read UAPI response: "errno=0\n\n"
        let mut response = [0u8; 64];
        let n = stream
            .read(&mut response)
            .await
            .map_err(|e| CoreError::Internal(format!("Failed to read UAPI response: {e}")))?;
        let resp_str = String::from_utf8_lossy(&response[..n]);
        if !resp_str.contains("errno=0") {
            let _ = child.kill().await;
            return Err(CoreError::Internal(format!(
                "UAPI configuration failed with response: {resp_str}"
            )));
        }

        // 3. Configure interface addresses
        for addr in &config.interface.address {
            let status = Command::new("ip")
                .args(["addr", "add", addr, "dev", &self.interface_name])
                .status()
                .await
                .map_err(|e| CoreError::Internal(format!("Failed to assign IP {addr}: {e}")))?;

            if !status.success() {
                warn!("Failed to add address {} to {}", addr, self.interface_name);
            }
        }

        // 4. Set MTU
        if let Some(mtu) = config.interface.mtu {
            let _ = Command::new("ip")
                .args([
                    "link",
                    "set",
                    "dev",
                    &self.interface_name,
                    "mtu",
                    &mtu.to_string(),
                ])
                .status()
                .await;
        }

        // 5. Bring interface UP
        let status = Command::new("ip")
            .args(["link", "set", "dev", &self.interface_name, "up"])
            .status()
            .await
            .map_err(|e| CoreError::Internal(format!("Failed to bring interface up: {e}")))?;

        if !status.success() {
            let _ = child.kill().await;
            return Err(CoreError::Internal(format!(
                "Failed to bring up interface {}",
                self.interface_name
            )));
        }

        journal.record(JournalAction::CreatedInterface {
            name: self.interface_name.clone(),
        });

        self.child = Some(child);
        self.is_running = true;
        self.last_config = Some(config.clone());
        self.cached_stats.endpoint = config.peer.endpoint.clone();

        info!("AwgBackend successfully started on {}", self.interface_name);
        Ok(())
    }

    async fn stop(&mut self, dry_run: bool, _journal: &mut UndoJournal) -> Result<()> {
        info!(
            "Stopping AwgBackend on interface '{}' (dry_run: {})",
            self.interface_name, dry_run
        );

        if dry_run {
            self.is_running = false;
            return Ok(());
        }

        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }

        // Delete network interface
        let _ = Command::new("ip")
            .args(["link", "del", "dev", &self.interface_name])
            .status()
            .await;

        // Clean up UAPI socket
        let socket_path = self.uapi_socket_path();
        if socket_path.exists() {
            let _ = tokio::fs::remove_file(socket_path).await;
        }

        self.last_config.take();
        self.is_running = false;
        Ok(())
    }

    async fn stats(&self) -> Result<TunnelStats> {
        let mut stats = self.cached_stats.clone();

        let rx_path = format!("/sys/class/net/{}/statistics/rx_bytes", self.interface_name);
        let tx_path = format!("/sys/class/net/{}/statistics/tx_bytes", self.interface_name);

        if let Ok(content) = tokio::fs::read_to_string(&rx_path).await {
            if let Ok(v) = content.trim().parse::<u64>() {
                stats.rx_bytes = v;
            }
        }

        if let Ok(content) = tokio::fs::read_to_string(&tx_path).await {
            if let Ok(v) = content.trim().parse::<u64>() {
                stats.tx_bytes = v;
            }
        }

        Ok(stats)
    }

    async fn is_running(&self) -> bool {
        self.is_running
    }
}
