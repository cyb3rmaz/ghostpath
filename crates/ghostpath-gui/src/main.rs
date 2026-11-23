#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use ghostpath_gui::ipc::commands::{handle_check_daemon_status, handle_rpc_invoke, AppState};
use ghostpath_gui::ipc::daemon_status::DaemonHealth;
use ghostpath_gui::ipc::events::spawn_event_loop;
use ghostpath_gui::single_instance::handle_second_instance;
use ghostpath_gui::tray::setup_system_tray;
use std::sync::Arc;
use tauri::Manager;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tauri::command]
async fn rpc_request(
    state: tauri::State<'_, Arc<AppState>>,
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    handle_rpc_invoke(&state, &method, params).await
}

#[tauri::command]
async fn check_daemon_status(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<DaemonHealth, String> {
    Ok(handle_check_daemon_status(&state).await)
}

fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting GhostPath VPN native desktop client");

    let app_state = Arc::new(AppState::default());
    let state_for_setup = app_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(handle_second_instance))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(app_state)
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // 1. Setup system tray
            let tray_available = setup_system_tray(&app_handle).unwrap_or(false);

            // 2. Setup close-to-hide window behavior
            if let Some(window) = app.get_webview_window("main") {
                let win_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if tray_available {
                            api.prevent_close();
                            let _ = win_clone.hide();
                            info!("Window hidden to system tray");
                        }
                    }
                });
            }

            // 3. Start background IPC event streaming loop
            spawn_event_loop(app_handle, state_for_setup.ipc_client.clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![rpc_request, check_daemon_status])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("Error running Tauri application: {}", e))?;

    Ok(())
}
