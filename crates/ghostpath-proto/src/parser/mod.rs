pub mod ss;
pub mod subscription;
pub mod trojan;
pub mod vless;
pub mod vmess;

use ghostpath_core::models::ServerNode;
use ghostpath_core::{CoreError, Result};

pub fn parse_any_link(raw: &str) -> Result<ServerNode> {
    let trimmed = raw.trim();
    if trimmed.starts_with("vless://") {
        let cfg = vless::parse_vless_uri(trimmed)?;
        Ok(cfg.to_server_node(trimmed))
    } else if trimmed.starts_with("vmess://") {
        let cfg = vmess::parse_vmess_uri(trimmed)?;
        Ok(cfg.to_server_node(trimmed))
    } else if trimmed.starts_with("trojan://") {
        let cfg = trojan::parse_trojan_uri(trimmed)?;
        Ok(cfg.to_server_node(trimmed))
    } else if trimmed.starts_with("ss://") {
        let cfg = ss::parse_ss_uri(trimmed)?;
        Ok(cfg.to_server_node(trimmed))
    } else {
        Err(CoreError::Config(format!(
            "Unsupported protocol URI scheme in '{trimmed}'"
        )))
    }
}
