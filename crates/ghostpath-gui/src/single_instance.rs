use crate::deeplink::parse_vpn_deep_link;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tracing::{info, warn};

static LAST_DEEP_LINK: Mutex<Option<Instant>> = Mutex::new(None);
const DEEP_LINK_COOLDOWN: Duration = Duration::from_millis(500);
const MAX_LINKS_PER_LAUNCH: usize = 3;

pub fn handle_second_instance(app: &AppHandle, argv: Vec<String>, _cwd: String) {
    info!("Focusing existing GhostPath window from second instance launch");

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }

    let mut emitted = 0usize;
    for arg in argv {
        if emitted >= MAX_LINKS_PER_LAUNCH {
            warn!("Rate-limited excess deep links in single-instance launch");
            break;
        }
        if let Some(link) = parse_vpn_deep_link(&arg) {
            let allow = if let Ok(mut guard) = LAST_DEEP_LINK.lock() {
                let now = Instant::now();
                if guard.is_some_and(|prev| now.duration_since(prev) < DEEP_LINK_COOLDOWN) {
                    false
                } else {
                    *guard = Some(now);
                    true
                }
            } else {
                true
            };
            if allow {
                info!(protocol = %link.protocol, "Received deep link in secondary instance");
                let _ = app.emit("deep_link_imported", link.raw);
                emitted += 1;
            } else {
                warn!("Dropped deep link due to cooldown rate limit");
            }
        }
    }
}
