use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod mock;

#[derive(Parser, Debug)]
#[command(
    name = "ghostpathd",
    about = "Privileged system daemon for GhostPath VPN"
)]
struct DaemonCli {
    /// Path to config file override
    #[arg(short, long)]
    config: Option<std::path::PathBuf>,

    /// Run cleanup/recovery pass and exit
    #[arg(long)]
    recover: bool,

    /// Dry run mode: print actions without applying network changes
    #[arg(long)]
    dry_run: bool,

    /// Run as a mock JSON-RPC backend for frontend UI testing
    #[arg(long)]
    mock: bool,

    /// Port for mock JSON-RPC server (default: 3001)
    #[arg(long, default_value = "3001")]
    port: u16,
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

    tracing::info!("Starting GhostPath VPN daemon (ghostpathd)");

    if args.mock {
        tracing::info!(port = args.port, "Starting mock daemon for UI testing");
        let server = mock::MockDaemonServer::new(args.port);
        server.run().await?;
        return Ok(());
    }

    if args.recover {
        tracing::info!("Running recovery pass for stale state...");
        if let Some(journal) = ghostpath_net::UndoJournal::load()? {
            tracing::info!(
                count = journal.actions.len(),
                "Found undo journal actions to revert"
            );
            ghostpath_net::UndoJournal::remove_journal_file()?;
            tracing::info!("Undo journal cleared");
        } else {
            tracing::info!("No stale undo journal found");
        }
        return Ok(());
    }

    if args.dry_run {
        tracing::info!("Dry-run mode activated: skipping system mutation");
        return Ok(());
    }

    tracing::info!("GhostPath VPN daemon initialized successfully");
    Ok(())
}
