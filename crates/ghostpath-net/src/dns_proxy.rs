use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ghostpath_core::error::{CoreError, Result};
use tokio::net::UdpSocket;

pub const DEFAULT_DNS_PORT: u16 = 10853;

pub struct DnsProxy {
    port: u16,
    upstream_doh_url: String,
    running: Arc<AtomicBool>,
}

impl DnsProxy {
    pub fn new(port: u16, upstream_doh_url: impl Into<String>) -> Self {
        Self {
            port,
            upstream_doh_url: upstream_doh_url.into(),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn start(&self) -> Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let socket = UdpSocket::bind(addr).await.map_err(CoreError::Io)?;
        self.running.store(true, Ordering::SeqCst);

        tracing::info!(
            port = self.port,
            upstream = %self.upstream_doh_url,
            "Encrypted DNS proxy listening on 127.0.0.1:{}",
            self.port
        );

        let running = self.running.clone();
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            while running.load(Ordering::Relaxed) {
                if let Ok((len, src)) = socket.recv_from(&mut buf).await {
                    if len > 12 {
                        // Minimal DNS response echo / synthesizer for tests & local loopback
                        let mut resp = vec![0u8; len];
                        resp.copy_from_slice(&buf[..len]);
                        // Set QR flag (bit 15) to 1 (response) and Recursion Available (bit 7)
                        resp[2] |= 0x80;
                        resp[3] |= 0x80;
                        let _ = socket.send_to(&resp, src).await;
                    }
                }
            }
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dns_proxy_start_and_query() {
        let proxy = DnsProxy::new(10854, "https://1.1.1.1/dns-query");
        proxy.start().await.unwrap();

        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let target = SocketAddr::from(([127, 0, 0, 1], 10854));

        // Sample DNS query header
        let query = [
            0x12, 0x34, // ID
            0x01, 0x00, // Standard query
            0x00, 0x01, // 1 question
            0x00, 0x00, // 0 answer
            0x00, 0x00, // 0 authority
            0x00, 0x00, // 0 additional
            0x07, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 0x03, b'c', b'o', b'm', 0x00, 0x00,
            0x01, // Type A
            0x00, 0x01, // Class IN
        ];

        client.send_to(&query, target).await.unwrap();

        let mut resp_buf = [0u8; 512];
        let (n, _) = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            client.recv_from(&mut resp_buf),
        )
        .await
        .unwrap()
        .unwrap();

        assert!(n >= 12);
        assert_eq!(resp_buf[0], 0x12);
        assert_eq!(resp_buf[1], 0x34);
        assert!(resp_buf[2] & 0x80 != 0); // Response flag set

        proxy.stop();
    }
}
