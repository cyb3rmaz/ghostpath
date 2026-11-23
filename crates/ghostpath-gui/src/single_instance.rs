use crate::deeplink::parse_vpn_deep_link;
use tauri::{AppHandle, Emitter, Manager};
use tracing::info;

pub fn handle_second_instance(app: &AppHandle, argv: Vec<String>, _cwd: String) {
    info!(args = ?argv, "Focusing existing GhostPath window from second instance launch");

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }

    for arg in argv {
        if let Some(link) = parse_vpn_deep_link(&arg) {
            info!(protocol = %link.protocol, "Received deep link in secondary instance");
            let _ = app.emit("deep_link_imported", link.raw);
        }
    }
}
