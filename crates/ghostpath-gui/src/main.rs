#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use ghostpath_gui::ipc::commands::{handle_rpc_invoke, AppState};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tauri::command]
async fn rpc_request(
    state: tauri::State<'_, Arc<AppState>>,
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    handle_rpc_invoke(&state.mock_daemon, &method, params).await
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

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![rpc_request])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("Error running Tauri application: {}", e))?;

    Ok(())
}
