use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroize;

/// Error types related to AmneziaVPN and AmneziaWG parsing, decoding, and validation.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum AmneziaError {
    #[error("Base64 decoding failed: {0}")]
    InvalidBase64(String),

    #[error("Invalid qCompress header: expected at least 4 bytes for length prefix")]
    InvalidHeader,

    #[error("Uncompressed size {size} exceeds maximum safe limit of {max_size} bytes")]
    DecompressionSizeExceeded { size: usize, max_size: usize },

    #[error("Zlib decompression failed: {0}")]
    DecompressionFailed(String),

    #[error("Failed to parse JSON: {0}")]
    InvalidJson(String),

    #[error("Invalid INI syntax: {0}")]
    InvalidIni(String),

    #[error("Missing required field: {0}")]
    MissingRequiredField(String),

    #[error("Invalid obfuscation parameter: {0}")]
    InvalidObfuscation(String),

    #[error("Invalid cryptographic key: expected 32 bytes (44 chars base64), got {0} chars")]
    InvalidKeyLength(usize),

    #[error("Unsupported container: {container_type} ({reason})")]
    UnsupportedContainer {
        container_type: String,
        reason: String,
    },
}

/// AmneziaWG obfuscation parameters for WireGuard packets (v1 and v2 specs).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwgObfuscationConfig {
    /// Junk packet count sent before handshake (typically 4..12)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jc: Option<u16>,

    /// Minimum junk packet size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jmin: Option<u16>,

    /// Maximum junk packet size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jmax: Option<u16>,

    /// Init packet padding size (bytes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s1: Option<u16>,

    /// Response packet padding size (bytes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s2: Option<u16>,

    /// Cookie packet padding size (bytes, v2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<u16>,

    /// Transport packet padding size (bytes, v2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s4: Option<u16>,

    /// Init packet header magic / message type override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h1: Option<u32>,

    /// Response packet header magic / message type override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h2: Option<u32>,

    /// Cookie packet header magic / message type override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h3: Option<u32>,

    /// Transport packet header magic / message type override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h4: Option<u32>,

    /// Signature packet 1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i1: Option<String>,

    /// Signature packet 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i2: Option<String>,

    /// Signature packet 3
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i3: Option<String>,

    /// Signature packet 4
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i4: Option<String>,

    /// Signature packet 5
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i5: Option<String>,
}

impl AwgObfuscationConfig {
    /// Returns true if any obfuscation parameter is active.
    pub fn is_obfuscated(&self) -> bool {
        self.jc.is_some()
            || self.jmin.is_some()
            || self.jmax.is_some()
            || self.s1.is_some()
            || self.s2.is_some()
            || self.s3.is_some()
            || self.s4.is_some()
            || self.h1.is_some()
            || self.h2.is_some()
            || self.h3.is_some()
            || self.h4.is_some()
            || self.i1.is_some()
            || self.i2.is_some()
            || self.i3.is_some()
            || self.i4.is_some()
            || self.i5.is_some()
    }

    /// Validates obfuscation parameter constraints.
    pub fn validate(&self) -> Result<(), AmneziaError> {
        if let (Some(jmin), Some(jmax)) = (self.jmin, self.jmax) {
            if jmin > jmax {
                return Err(AmneziaError::InvalidObfuscation(format!(
                    "Jmin cannot be greater than Jmax: {jmin} > {jmax}"
                )));
            }
        }

        // Validate H1..H4 uniqueness if provided
        let mut headers = Vec::new();
        if let Some(h) = self.h1 {
            headers.push(("H1", h));
        }
        if let Some(h) = self.h2 {
            headers.push(("H2", h));
        }
        if let Some(h) = self.h3 {
            headers.push(("H3", h));
        }
        if let Some(h) = self.h4 {
            headers.push(("H4", h));
        }

        for i in 0..headers.len() {
            for j in (i + 1)..headers.len() {
                if headers[i].1 == headers[j].1 {
                    return Err(AmneziaError::InvalidObfuscation(format!(
                        "H1..H4 magic headers must be pairwise distinct: {} and {} both equal {}",
                        headers[i].0, headers[j].0, headers[i].1
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Interface settings for an AmneziaWG or WireGuard tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwgInterfaceConfig {
    /// Base64 encoded 32-byte Curve25519 private key
    pub private_key: String,

    /// Interface CIDR addresses (IPv4 and/or IPv6)
    pub address: Vec<String>,

    /// DNS resolver IP addresses
    pub dns: Vec<String>,

    /// Maximum transmission unit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u16>,

    /// Obfuscation settings (empty if standard WireGuard)
    #[serde(default)]
    pub obfuscation: AwgObfuscationConfig,
}

impl Zeroize for AwgInterfaceConfig {
    fn zeroize(&mut self) {
        self.private_key.zeroize();
    }
}

impl Drop for AwgInterfaceConfig {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Peer connection settings for an AmneziaWG or WireGuard endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwgPeerConfig {
    /// Base64 encoded 32-byte Curve25519 public key
    pub public_key: String,

    /// Optional Base64 encoded 32-byte preshared key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preshared_key: Option<String>,

    /// Hostname or IP with port (e.g. "203.0.113.10:39001")
    pub endpoint: String,

    /// Routed destination CIDRs (default ["0.0.0.0/0", "::/0"])
    pub allowed_ips: Vec<String>,

    /// Keepalive interval in seconds (typical 25)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent_keepalive: Option<u16>,
}

impl Zeroize for AwgPeerConfig {
    fn zeroize(&mut self) {
        if let Some(ref mut psk) = self.preshared_key {
            psk.zeroize();
        }
    }
}

impl Drop for AwgPeerConfig {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Complete AmneziaWG / WireGuard profile configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwgProfileConfig {
    pub id: String,
    pub name: String,
    pub interface: AwgInterfaceConfig,
    pub peer: AwgPeerConfig,
}

impl AwgProfileConfig {
    /// Returns true if this profile utilizes AmneziaWG obfuscations.
    pub fn is_amnezia(&self) -> bool {
        self.interface.obfuscation.is_obfuscated()
    }

    /// Validates profile keys and parameters.
    pub fn validate(&self) -> Result<(), AmneziaError> {
        if self.interface.private_key.trim().is_empty() {
            return Err(AmneziaError::MissingRequiredField("PrivateKey".into()));
        }
        if self.peer.public_key.trim().is_empty() {
            return Err(AmneziaError::MissingRequiredField("PublicKey".into()));
        }
        if self.peer.endpoint.trim().is_empty() {
            return Err(AmneziaError::MissingRequiredField("Endpoint".into()));
        }

        self.interface.obfuscation.validate()?;
        Ok(())
    }
}

/// Supported and unsupported container types in AmneziaVPN servers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AmneziaContainerType {
    Awg,
    WireGuard,
    Xray,
    Shadowsocks,
    OpenVpnCloak,
    OpenVpn,
    Ipsec,
    Unknown(String),
}

impl From<&str> for AmneziaContainerType {
    fn from(s: &str) -> Self {
        match s {
            "amnezia-awg" | "awg" => Self::Awg,
            "amnezia-wireguard" | "wireguard" => Self::WireGuard,
            "amnezia-xray" | "xray" => Self::Xray,
            "amnezia-shadowsocks" | "shadowsocks" => Self::Shadowsocks,
            "amnezia-openvpn-cloak" | "openvpn-cloak" => Self::OpenVpnCloak,
            "amnezia-openvpn" | "openvpn" => Self::OpenVpn,
            "amnezia-ipsec" | "ipsec" => Self::Ipsec,
            other => Self::Unknown(other.to_string()),
        }
    }
}

/// An imported container item from an Amnezia server definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmneziaContainer {
    pub container_name: String,
    pub container_type: AmneziaContainerType,
    pub is_supported: bool,
    pub unsupported_reason: Option<String>,
    pub awg_profile: Option<AwgProfileConfig>,
    pub raw_json: serde_json::Value,
}

/// Amnezia Server representation decoded from a `vpn://` key or `.vpn` export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmneziaServerConfig {
    pub server_name: String,
    pub host_name: String,
    pub default_container: Option<String>,
    pub dns: Vec<String>,
    pub containers: Vec<AmneziaContainer>,
}
