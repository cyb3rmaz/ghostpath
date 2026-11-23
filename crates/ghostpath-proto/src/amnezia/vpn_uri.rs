use super::ini_parser::{export_awg_conf, parse_awg_conf};
use super::model::{
    AmneziaContainer, AmneziaContainerType, AmneziaError, AmneziaServerConfig,
};
use base64::prelude::*;
use serde_json::Value;

pub const MAX_UNCOMPRESSED_SIZE: usize = 2 * 1024 * 1024; // 2 MB limit to prevent zip bombs

/// Decodes an Amnezia `vpn://` URI into an `AmneziaServerConfig`.
pub fn parse_vpn_uri(uri: &str) -> Result<AmneziaServerConfig, AmneziaError> {
    let trimmed = uri.trim();
    if !trimmed.to_lowercase().starts_with("vpn://") {
        return Err(AmneziaError::InvalidBase64(
            "URI scheme must start with 'vpn://'".to_string(),
        ));
    }

    let payload_b64 = &trimmed[6..];
    if payload_b64.is_empty() {
        return Err(AmneziaError::InvalidBase64("Empty payload in vpn:// URI".to_string()));
    }

    // Decode URL-safe Base64 (supporting both padded and unpadded variants)
    let raw_bytes = BASE64_URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| BASE64_URL_SAFE.decode(payload_b64))
        .or_else(|_| BASE64_STANDARD.decode(payload_b64))
        .map_err(|e| AmneziaError::InvalidBase64(e.to_string()))?;

    if raw_bytes.len() < 4 {
        return Err(AmneziaError::InvalidHeader);
    }

    // Qt qCompress format: 4-byte Big-Endian uncompressed length
    let uncompressed_len = u32::from_be_bytes([
        raw_bytes[0],
        raw_bytes[1],
        raw_bytes[2],
        raw_bytes[3],
    ]) as usize;

    if uncompressed_len > MAX_UNCOMPRESSED_SIZE {
        return Err(AmneziaError::DecompressionSizeExceeded {
            size: uncompressed_len,
            max_size: MAX_UNCOMPRESSED_SIZE,
        });
    }

    // Decompress the remaining zlib deflate stream with hard limit
    let compressed_stream = &raw_bytes[4..];
    let decompressed = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(
        compressed_stream,
        MAX_UNCOMPRESSED_SIZE,
    )
    .map_err(|e| AmneziaError::DecompressionFailed(format!("{e:?}")))?;

    if decompressed.len() != uncompressed_len {
        return Err(AmneziaError::DecompressionFailed(format!(
            "Decompressed length mismatch: expected {} bytes, got {}",
            uncompressed_len,
            decompressed.len()
        )));
    }

    let root: Value = serde_json::from_slice(&decompressed)
        .map_err(|e| AmneziaError::InvalidJson(e.to_string()))?;

    parse_server_json(&root)
}

/// Encodes an `AmneziaServerConfig` into a `vpn://` URI.
pub fn encode_vpn_uri(server: &AmneziaServerConfig) -> Result<String, AmneziaError> {
    let mut containers_json = Vec::new();

    for c in &server.containers {
        let mut container_obj = serde_json::Map::new();
        container_obj.insert("container".to_string(), Value::String(c.container_name.clone()));

        if let Some(ref awg) = c.awg_profile {
            let conf_str = export_awg_conf(awg);
            let mut awg_data = serde_json::Map::new();
            awg_data.insert("last_config".to_string(), Value::String(conf_str));
            container_obj.insert("awg".to_string(), Value::Object(awg_data));
        } else if !c.raw_json.is_null() {
            let key = match c.container_type {
                AmneziaContainerType::Awg => "awg",
                AmneziaContainerType::WireGuard => "wireguard",
                AmneziaContainerType::Xray => "xray",
                AmneziaContainerType::Shadowsocks => "shadowsocks",
                AmneziaContainerType::OpenVpnCloak => "openvpn-cloak",
                AmneziaContainerType::OpenVpn => "openvpn",
                AmneziaContainerType::Ipsec => "ipsec",
                AmneziaContainerType::Unknown(ref k) => k.as_str(),
            };
            container_obj.insert(key.to_string(), c.raw_json.clone());
        }

        containers_json.push(Value::Object(container_obj));
    }

    let mut root_map = serde_json::Map::new();
    root_map.insert("containers".to_string(), Value::Array(containers_json));
    root_map.insert(
        "description".to_string(),
        Value::String(server.server_name.clone()),
    );
    root_map.insert(
        "hostName".to_string(),
        Value::String(server.host_name.clone()),
    );
    if let Some(ref def) = server.default_container {
        root_map.insert("defaultContainer".to_string(), Value::String(def.clone()));
    }
    if let Some(first_dns) = server.dns.first() {
        root_map.insert("dns1".to_string(), Value::String(first_dns.clone()));
    }
    if let Some(second_dns) = server.dns.get(1) {
        root_map.insert("dns2".to_string(), Value::String(second_dns.clone()));
    }

    let json_bytes = serde_json::to_vec(&Value::Object(root_map))
        .map_err(|e| AmneziaError::InvalidJson(e.to_string()))?;

    let uncompressed_len = json_bytes.len() as u32;
    let compressed_bytes = miniz_oxide::deflate::compress_to_vec_zlib(&json_bytes, 8);

    let mut payload = Vec::with_capacity(4 + compressed_bytes.len());
    payload.extend_from_slice(&uncompressed_len.to_be_bytes());
    payload.extend_from_slice(&compressed_bytes);

    let b64 = BASE64_URL_SAFE_NO_PAD.encode(&payload);
    Ok(format!("vpn://{b64}"))
}

fn parse_server_json(root: &Value) -> Result<AmneziaServerConfig, AmneziaError> {
    let server_name = root
        .get("description")
        .or_else(|| root.get("serverName"))
        .and_then(|v| v.as_str())
        .unwrap_or("Amnezia Server")
        .to_string();

    let host_name = root
        .get("hostName")
        .or_else(|| root.get("ip1"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let default_container = root
        .get("defaultContainer")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut dns = Vec::new();
    if let Some(d1) = root.get("dns1").and_then(|v| v.as_str()) {
        if !d1.trim().is_empty() {
            dns.push(d1.trim().to_string());
        }
    }
    if let Some(d2) = root.get("dns2").and_then(|v| v.as_str()) {
        if !d2.trim().is_empty() {
            dns.push(d2.trim().to_string());
        }
    }

    let containers_array = root
        .get("containers")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut containers = Vec::new();

    for c_val in containers_array {
        let container_name = c_val
            .get("container")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let container_type = AmneziaContainerType::from(container_name.as_str());

        let is_supported;
        let mut unsupported_reason = None;
        let mut awg_profile = None;

        match &container_type {
            AmneziaContainerType::Awg | AmneziaContainerType::WireGuard => {
                is_supported = true;
                // Attempt to extract AWG config from awg/wireguard node
                let container_data = c_val.get(&container_name).or_else(|| {
                    if container_name.contains("awg") {
                        c_val.get("awg")
                    } else {
                        c_val.get("wireguard")
                    }
                });

                if let Some(c_data) = container_data {
                    if let Some(conf_str) = extract_conf_string(c_data) {
                        let prof_name = format!("{server_name} ({container_name})");
                        if let Ok(prof) = parse_awg_conf(&conf_str, &prof_name) {
                            awg_profile = Some(prof);
                        }
                    }
                }
            }
            AmneziaContainerType::Xray => {
                is_supported = true;
            }
            AmneziaContainerType::Shadowsocks => {
                is_supported = true;
            }
            AmneziaContainerType::OpenVpnCloak => {
                is_supported = false;
                unsupported_reason = Some(
                    "OpenVPN over Cloak is not supported in GhostPath. Use AmneziaWG or VLESS Reality instead."
                        .to_string(),
                );
            }
            AmneziaContainerType::OpenVpn => {
                is_supported = false;
                unsupported_reason = Some(
                    "Plain OpenVPN container is not supported in GhostPath. Use AmneziaWG instead."
                        .to_string(),
                );
            }
            AmneziaContainerType::Ipsec => {
                is_supported = false;
                unsupported_reason = Some(
                    "IPsec container is not supported in GhostPath. Use AmneziaWG instead."
                        .to_string(),
                );
            }
            AmneziaContainerType::Unknown(ref name) => {
                is_supported = false;
                unsupported_reason = Some(format!("Unknown container '{name}' is not supported"));
            }
        }

        containers.push(AmneziaContainer {
            container_name,
            container_type,
            is_supported,
            unsupported_reason,
            awg_profile,
            raw_json: c_val,
        });
    }

    Ok(AmneziaServerConfig {
        server_name,
        host_name,
        default_container,
        dns,
        containers,
    })
}

fn extract_conf_string(val: &Value) -> Option<String> {
    // 1. Direct "config": "[Interface]..."
    if let Some(s) = val.get("config").and_then(|v| v.as_str()) {
        if s.contains("[Interface]") {
            return Some(s.to_string());
        }
    }

    // 2. "last_config": "{\"config\":\"[Interface]...\"}"
    if let Some(lc) = val.get("last_config").and_then(|v| v.as_str()) {
        if lc.contains("[Interface]") {
            // It could be raw INI
            return Some(lc.to_string());
        }
        // Or it could be JSON-encoded string
        if let Ok(nested) = serde_json::from_str::<Value>(lc) {
            if let Some(s) = nested.get("config").and_then(|v| v.as_str()) {
                if s.contains("[Interface]") {
                    return Some(s.to_string());
                }
            }
        }
    }

    None
}
