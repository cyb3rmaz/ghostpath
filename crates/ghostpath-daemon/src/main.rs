use anyhow::Result;
use clap::Parser;
use ghostpath_daemon::engine::DaemonEngine;
use ghostpath_daemon::ipc::IpcServer;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(
    name = "ghostpathd",
    about = "Privileged system daemon for GhostPath VPN"
)]
struct DaemonCli {
    /// Path to config file override
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Path to Unix domain socket (default: /run/ghostpath/ghostpathd.sock)
    #[arg(short, long)]
    socket: Option<PathBuf>,

    /// Run cleanup/recovery pass and exit
    #[arg(long)]
    recover: bool,

    /// Dry run mode: print actions without applying network changes
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = DaemonCli::parse();

    info!("Starting GhostPath VPN daemon (ghostpathd)");

    if args.recover {
        info!("Running recovery pass for stale state...");
        if let Some(journal) = ghostpath_net::UndoJournal::load()? {
            info!(
                count = journal.actions.len(),
                "Found undo journal actions to revert"
            );
            ghostpath_net::UndoJournal::remove_journal_file()?;
            info!("Undo journal cleared");
        } else {
            info!("No stale undo journal found");
        }
        return Ok(());
    }

    let socket_path = args
        .socket
        .or_else(|| std::env::var("GHOSTPATH_SOCKET").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/run/ghostpath/ghostpathd.sock"));

    let engine = Arc::new(DaemonEngine::new(args.dry_run));
    let server = IpcServer::new(socket_path, engine.clone());

    info!(
        dry_run = args.dry_run,
        "GhostPath daemon engine initialized successfully"
    );

    // Setup graceful shutdown handlers
    let engine_shutdown = engine.clone();
    tokio::select! {
        res = server.run() => {
            if let Err(e) = res {
                tracing::error!("IPC server encountered error: {}", e);
            }
        }
        _ = wait_for_shutdown() => {
            info!("Shutdown signal received, disconnecting and restoring state...");
            let _ = engine_shutdown.disconnect().await;
            server.cleanup();
            info!("GhostPath daemon shutdown complete");
        }
    }

    Ok(())
}

async fn wait_for_shutdown() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");
        let mut sigint =
            signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT");
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C");
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
        info!("Received Ctrl+C");
    }
}
