use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tracing::{info, warn};

pub fn setup_system_tray(app: &AppHandle) -> Result<bool, Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "Open GhostPath VPN", true, None::<&str>)?;
    let toggle_item = MenuItem::with_id(
        app,
        "toggle_connect",
        "Toggle Connection",
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = match Menu::with_items(app, &[&show_item, &toggle_item, &quit_item]) {
        Ok(m) => m,
        Err(e) => {
            warn!(
                "Could not construct tray menu: {}. Fallback to window mode.",
                e
            );
            return Ok(false);
        }
    };

    let icon = match app.default_window_icon() {
        Some(i) => i.clone(),
        None => {
            warn!("Default window icon missing for tray");
            return Ok(false);
        }
    };

    let tray_res = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("GhostPath VPN — Linux Tunnel")
        .icon(icon)
        .on_menu_event(move |app_handle, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
            "toggle_connect" => {
                let _ = app_handle.emit("tray_toggle_connect", ());
            }
            "quit" => {
                info!("Quit requested from system tray");
                app_handle.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app);

    match tray_res {
        Ok(_) => {
            info!("System tray icon registered successfully");
            Ok(true)
        }
        Err(e) => {
            warn!(
                "System tray is unavailable on this desktop environment: {}. Window fallback active.",
                e
            );
            Ok(false)
        }
    }
}
