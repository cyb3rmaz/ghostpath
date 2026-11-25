use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use ghostpath_core::error::{CoreError, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::Semaphore;

pub const DEFAULT_DNS_PORT: u16 = 10853;
const MAX_INFLIGHT_DNS_QUERIES: usize = 128;
const DNS_UPSTREAM_TIMEOUT: Duration = Duration::from_secs(4);

pub struct DnsProxy {
    port: u16,
    upstream_doh_url: String,
    socks_port: Option<u16>,
    upstream_tcp_addr: SocketAddr,
    running: Arc<AtomicBool>,
}

impl DnsProxy {
    pub fn new(port: u16, upstream_doh_url: impl Into<String>) -> Self {
        Self {
            port,
            upstream_doh_url: upstream_doh_url.into(),
            socks_port: None,
            upstream_tcp_addr: SocketAddr::from((Ipv4Addr::new(1, 1, 1, 1), 53)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    #[must_use]
    pub fn with_socks_upstream(mut self, socks_port: u16, upstream_dns_ip: Ipv4Addr) -> Self {
        self.socks_port = Some(socks_port);
        self.upstream_tcp_addr = SocketAddr::from((upstream_dns_ip, 53));
        self
    }

    #[must_use]
    pub fn with_direct_tcp_upstream(mut self, addr: SocketAddr) -> Self {
        self.socks_port = None;
        self.upstream_tcp_addr = addr;
        self
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn start(&self) -> Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let socket = Arc::new(UdpSocket::bind(addr).await.map_err(CoreError::Io)?);
        self.running.store(true, Ordering::SeqCst);

        tracing::info!(
            port = self.port,
            upstream = %self.upstream_doh_url,
            socks_port = ?self.socks_port,
            "Encrypted DNS-over-TCP/SOCKS5 proxy listening on 127.0.0.1:{}",
            self.port
        );

        let running = self.running.clone();
        let socks_port = self.socks_port;
        let upstream_addr = self.upstream_tcp_addr;
        let sem = Arc::new(Semaphore::new(MAX_INFLIGHT_DNS_QUERIES));

        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            while running.load(Ordering::Relaxed) {
                match tokio::time::timeout(Duration::from_millis(250), socket.recv_from(&mut buf))
                    .await
                {
                    Ok(Ok((len, src))) if len >= 12 => {
                        let Ok(permit) = sem.clone().try_acquire_owned() else {
                            continue;
                        };
                        let query = buf[..len].to_vec();
                        let sock = socket.clone();
                        tokio::spawn(async move {
                            let _permit = permit;
                            let resp = match tokio::time::timeout(
                                DNS_UPSTREAM_TIMEOUT,
                                forward_dns_tcp(&query, socks_port, upstream_addr),
                            )
                            .await
                            {
                                Ok(Ok(r)) => r,
                                _ => build_servfail(&query),
                            };
                            let _ = sock.send_to(&resp, src).await;
                        });
                    }
                    Ok(Err(_)) => break,
                    _ => {}
                }
            }
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

async fn forward_dns_tcp(
    query: &[u8],
    socks_port: Option<u16>,
    upstream_addr: SocketAddr,
) -> std::io::Result<Vec<u8>> {
    let mut stream = if let Some(sp) = socks_port {
        let mut s = TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, sp))).await?;
        // SOCKS5 greeting: ver 5, 1 method, no-auth
        s.write_all(&[0x05, 0x01, 0x00]).await?;
        let mut method_resp = [0u8; 2];
        s.read_exact(&mut method_resp).await?;
        if method_resp != [0x05, 0x00] {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "SOCKS5 auth rejected",
            ));
        }
        // SOCKS5 CONNECT to upstream IPv4:port
        let octets = match upstream_addr.ip() {
            std::net::IpAddr::V4(v4) => v4.octets(),
            std::net::IpAddr::V6(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "IPv6 upstream not supported in SOCKS5 DNS helper",
                ))
            }
        };
        let port = upstream_addr.port().to_be_bytes();
        let req = [
            0x05, 0x01, 0x00, 0x01, octets[0], octets[1], octets[2], octets[3], port[0], port[1],
        ];
        s.write_all(&req).await?;
        let mut conn_resp = [0u8; 10];
        s.read_exact(&mut conn_resp).await?;
        if conn_resp[1] != 0x00 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "SOCKS5 CONNECT failed",
            ));
        }
        s
    } else {
        TcpStream::connect(upstream_addr).await?
    };

    let qlen = u16::try_from(query.len()).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "DNS query too large")
    })?;
    stream.write_all(&qlen.to_be_bytes()).await?;
    stream.write_all(query).await?;
    stream.flush().await?;

    let mut rlen_buf = [0u8; 2];
    stream.read_exact(&mut rlen_buf).await?;
    let rlen = u16::from_be_bytes(rlen_buf) as usize;
    if !(12..=4096).contains(&rlen) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid DNS TCP response length",
        ));
    }
    let mut resp = vec![0u8; rlen];
    stream.read_exact(&mut resp).await?;
    Ok(resp)
}

fn build_servfail(query: &[u8]) -> Vec<u8> {
    let mut resp = query.to_vec();
    if resp.len() >= 12 {
        resp[2] |= 0x80; // QR = 1 (response)
        resp[3] = (resp[3] & 0xF0) | 0x82; // RA = 1, RCODE = 2 (SERVFAIL)
        resp[6] = 0;
        resp[7] = 0;
        resp[8] = 0;
        resp[9] = 0;
        resp[10] = 0;
        resp[11] = 0;
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_dns_proxy_forwards_over_upstream_and_returns_servfail_when_down() {
        // 1. Start a local mock TCP DNS server returning a real A record (93.184.216.34)
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            if let Ok((mut conn, _)) = listener.accept().await {
                let mut len_buf = [0u8; 2];
                if conn.read_exact(&mut len_buf).await.is_ok() {
                    let qlen = u16::from_be_bytes(len_buf) as usize;
                    let mut q = vec![0u8; qlen];
                    if conn.read_exact(&mut q).await.is_ok() {
                        let mut r = q.clone();
                        r[2] |= 0x80; // QR=1
                        r[3] |= 0x80; // RA=1
                        r[7] = 1; // 1 answer
                        r.extend_from_slice(&[
                            0xC0, 0x0C, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3C, 0x00, 0x04,
                            93, 184, 216, 34,
                        ]);
                        let rlen = (r.len() as u16).to_be_bytes();
                        let _ = conn.write_all(&rlen).await;
                        let _ = conn.write_all(&r).await;
                    }
                }
            }
        });

        let proxy = DnsProxy::new(10854, "https://1.1.1.1/dns-query")
            .with_direct_tcp_upstream(upstream_addr);
        proxy.start().await.unwrap();

        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let target = SocketAddr::from(([127, 0, 0, 1], 10854));

        let query = [
            0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, b'e',
            b'x', b'a', b'm', b'p', b'l', b'e', 0x03, b'c', b'o', b'm', 0x00, 0x00, 0x01, 0x00,
            0x01,
        ];

        client.send_to(&query, target).await.unwrap();

        let mut resp_buf = [0u8; 512];
        let (n, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_from(&mut resp_buf))
            .await
            .unwrap()
            .unwrap();

        assert!(
            n > query.len(),
            "expected forwarded answer record, not echo"
        );
        assert_eq!(resp_buf[0], 0x12);
        assert_eq!(resp_buf[1], 0x34);
        assert_eq!(resp_buf[7], 1); // 1 answer
        assert_eq!(&resp_buf[n - 4..n], &[93, 184, 216, 34]);
        proxy.stop();

        // 2. Verify SERVFAIL when upstream is closed
        let closed_port = upstream_addr.port();
        let dead_proxy = DnsProxy::new(10855, "https://1.1.1.1/dns-query")
            .with_direct_tcp_upstream(SocketAddr::from((Ipv4Addr::LOCALHOST, closed_port)));
        dead_proxy.start().await.unwrap();
        client
            .send_to(&query, SocketAddr::from(([127, 0, 0, 1], 10855)))
            .await
            .unwrap();
        let (n2, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_from(&mut resp_buf))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(n2, query.len());
        assert_eq!(resp_buf[3] & 0x0F, 2, "expected SERVFAIL RCODE=2");
        dead_proxy.stop();
    }
}
