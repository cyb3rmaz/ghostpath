use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ghostpath_core::error::{CoreError, Result};
use tokio::io::unix::AsyncFd;

pub const DEFAULT_TUN_NAME: &str = "ghost0";
pub const DEFAULT_MTU: u16 = 1420;
pub const DEFAULT_IPV4: &str = "10.8.0.2/24";
pub const DEFAULT_IPV4_PEER: &str = "10.8.0.1";
pub const DEFAULT_IPV6: &str = "fd00:5180::2/64";

pub struct TunDevice {
    name: String,
    mtu: u16,
    async_fd: Option<AsyncFd<OwnedFd>>,
    is_mock: bool,
    closed: Arc<AtomicBool>,
}

impl TunDevice {
    pub fn new(name: impl Into<String>, mtu: u16) -> Self {
        Self {
            name: name.into(),
            mtu,
            async_fd: None,
            is_mock: false,
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mtu(&self) -> u16 {
        self.mtu
    }

    pub fn is_mock(&self) -> bool {
        self.is_mock
    }

    /// Opens the TUN device on Linux via /dev/net/tun
    pub async fn open(&mut self) -> Result<()> {
        tracing::info!(name = %self.name, mtu = self.mtu, "Opening TUN interface");

        #[cfg(target_os = "linux")]
        {
            match open_linux_tun(&self.name) {
                Ok(raw_fd) => {
                    configure_link_and_address(&self.name, self.mtu)?;
                    // SAFETY: raw_fd is a valid open file descriptor returned by open_linux_tun
                    let owned_fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };
                    // SAFETY: `owned_fd` is valid, uniquely owned, and managed by `AsyncFd`
                    let registered = unsafe { AsyncFd::register(owned_fd) }.map_err(|e| {
                        CoreError::Internal(format!("Failed to register TUN AsyncFd: {e}"))
                    })?;
                    self.async_fd = Some(registered);
                    self.is_mock = false;
                    tracing::info!(name = %self.name, "Native Linux TUN interface initialized and link is UP");
                    Ok(())
                }
                Err(e) => {
                    #[cfg(any(test, feature = "dev-mock"))]
                    {
                        tracing::warn!(error = %e, "Could not open /dev/net/tun; falling back to mock TUN device (dev-mock enabled)");
                        self.is_mock = true;
                        Ok(())
                    }
                    #[cfg(not(any(test, feature = "dev-mock")))]
                    {
                        tracing::error!(error = %e, "Could not open /dev/net/tun (lack of CAP_NET_ADMIN/root)");
                        Err(CoreError::Internal(format!(
                            "Failed to open TUN device: {e}. The daemon requires CAP_NET_ADMIN."
                        )))
                    }
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            #[cfg(any(test, feature = "dev-mock"))]
            {
                tracing::info!("Non-Linux platform: utilizing in-memory mock TUN device");
                self.is_mock = true;
                return Ok(());
            }
            #[cfg(not(any(test, feature = "dev-mock")))]
            {
                return Err(CoreError::Internal(
                    "TUN device is only supported on Linux".to_string(),
                ));
            }
        }
    }

    /// Reads an IP packet from the TUN interface
    pub async fn read_packet(&self, buf: &mut [u8]) -> Result<usize> {
        if self.is_mock {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            return Ok(0);
        }

        if let Some(ref async_fd) = self.async_fd {
            loop {
                let mut guard = async_fd.readable().await.map_err(CoreError::Io)?;

                let fd = guard.get_inner().as_raw_fd();
                // SAFETY: fd is valid and open, buf is a valid writable byte slice
                let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };

                if n >= 0 {
                    return Ok(n as usize);
                }

                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    guard.clear_ready();
                    continue;
                }
                return Err(CoreError::Io(err));
            }
        }

        Err(CoreError::Internal("TUN device is not open".into()))
    }

    /// Writes an IP packet to the TUN interface
    pub async fn write_packet(&self, buf: &[u8]) -> Result<usize> {
        if self.is_mock {
            return Ok(buf.len());
        }

        if let Some(ref async_fd) = self.async_fd {
            loop {
                let mut guard = async_fd.writable().await.map_err(CoreError::Io)?;

                let fd = guard.get_inner().as_raw_fd();
                // SAFETY: fd is valid and open, buf is a valid readable byte slice
                let n = unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len()) };

                if n >= 0 {
                    return Ok(n as usize);
                }

                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    guard.clear_ready();
                    continue;
                }
                return Err(CoreError::Io(err));
            }
        }

        Err(CoreError::Internal("TUN device is not open".into()))
    }

    pub async fn close(&mut self) -> Result<()> {
        if self.closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        tracing::info!(name = %self.name, "Closing TUN interface");
        self.async_fd.take();

        // Tear down link
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("ip")
                .args(["link", "set", "dev", &self.name, "down"])
                .output();
        }

        Ok(())
    }
}

impl Drop for TunDevice {
    fn drop(&mut self) {
        if !self.closed.load(Ordering::SeqCst) {
            self.async_fd.take();
        }
    }
}

#[cfg(target_os = "linux")]
fn open_linux_tun(name: &str) -> Result<RawFd> {
    const IFF_TUN: libc::c_short = 0x0001;
    const IFF_NO_PI: libc::c_short = 0x1000;
    const TUNSETIFF: libc::c_ulong = 0x400454ca;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/net/tun")
        .map_err(CoreError::Io)?;

    let raw_fd = file.as_raw_fd();

    #[repr(C)]
    struct IfReq {
        ifr_name: [libc::c_char; libc::IFNAMSIZ],
        ifr_flags: libc::c_short,
        padding: [u8; 22],
    }

    let mut ifr = IfReq {
        ifr_name: [0; libc::IFNAMSIZ],
        ifr_flags: IFF_TUN | IFF_NO_PI,
        padding: [0; 22],
    };

    let bytes = name.as_bytes();
    let len = bytes.len().min(libc::IFNAMSIZ - 1);
    for (i, &b) in bytes[..len].iter().enumerate() {
        ifr.ifr_name[i] = b as libc::c_char;
    }

    // SAFETY: ioctl TUNSETIFF on valid /dev/net/tun file descriptor with valid ifreq struct
    let res = unsafe { libc::ioctl(raw_fd, TUNSETIFF, &mut ifr) };
    if res < 0 {
        return Err(CoreError::Io(std::io::Error::last_os_error()));
    }

    // Keep file open by leaking the File wrapper and managing raw fd
    std::mem::forget(file);
    Ok(raw_fd)
}

#[cfg(target_os = "linux")]
fn configure_link_and_address(name: &str, mtu: u16) -> Result<()> {
    fn run_ip(args: &[&str]) -> Result<()> {
        let out = std::process::Command::new("ip")
            .args(args)
            .output()
            .map_err(CoreError::Io)?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(CoreError::Internal(format!(
                "ip {} failed: {}",
                args.join(" "),
                stderr.trim()
            )));
        }
        Ok(())
    }

    // 1. Set MTU
    let mtu_str = mtu.to_string();
    run_ip(&["link", "set", "dev", name, "mtu", &mtu_str])?;

    // 2. Assign IPv4 address: 10.8.0.2/24 peer 10.8.0.1
    run_ip(&[
        "addr",
        "add",
        DEFAULT_IPV4,
        "peer",
        DEFAULT_IPV4_PEER,
        "dev",
        name,
    ])?;

    // 3. Assign IPv6 address: fd00:5180::2/64 (non-fatal if IPv6 is disabled)
    if let Err(e) = run_ip(&["-6", "addr", "add", DEFAULT_IPV6, "dev", name]) {
        tracing::warn!(error = %e, "IPv6 address assignment failed (IPv6 may be disabled)");
    }

    // 4. Set link UP
    run_ip(&["link", "set", "dev", name, "up"])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tun_device_lifecycle() {
        let mut tun = TunDevice::new("ghost0", 1420);
        assert_eq!(tun.name(), "ghost0");
        assert_eq!(tun.mtu(), 1420);

        tun.open().await.unwrap();
        let mut buf = [0u8; 1500];
        let n = tun.read_packet(&mut buf).await.unwrap();
        assert_eq!(n, 0); // In mock or idle, non-blocking returns 0

        tun.close().await.unwrap();
    }
}
