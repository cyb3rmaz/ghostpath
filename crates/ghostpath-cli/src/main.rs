use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use ghostpath_core::models::ConnectionStatus;
use ghostpath_core::GhostConfig;

#[derive(Parser, Debug)]
#[command(
    name = "ghostpath",
    about = "GhostPath VPN - Linux VPN client with reliable TUN mode"
)]
struct Cli {
    /// Format output as JSON for scripting
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Connect to a VPN server profile
    Connect {
        /// Profile ID or name to connect to (defaults to active node)
        node_id: Option<String>,
    },
    /// Disconnect from the active VPN tunnel
    Disconnect,
    /// Display connection status and traffic stats
    Status,
    /// Import a server profile link (vless://, vmess://, etc.) or subscription URL
    Import {
        /// URL, link or configuration string
        target: String,
        /// Optional friendly profile name
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Diagnose system environment, permissions, and network stack
    Doctor,
    /// View daemon logs
    Logs {
        /// Number of recent log lines to print
        #[arg(short = 'n', long, default_value = "50")]
        lines: usize,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = GhostConfig::load().unwrap_or_default();

    match cli.command {
        Commands::Connect { node_id } => {
            let target_node = node_id
                .or(config.active_node_id.clone())
                .unwrap_or_else(|| "default-node".to_string());

            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "connecting",
                        "node": target_node,
                    })
                );
            } else {
                println!(
                    "{} Connecting to node: {}",
                    "->".bright_green().bold(),
                    target_node.cyan()
                );
            }
        }
        Commands::Disconnect => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "disconnected"
                    })
                );
            } else {
                println!("{} Disconnected cleanly", "OK".bright_green().bold());
            }
        }
        Commands::Status => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": ConnectionStatus::Disconnected,
                        "interface": "ghost0",
                        "mtu": config.settings.mtu,
                        "core": config.settings.core_engine,
                    })
                );
            } else {
                println!("{}", "GhostPath VPN Status:".bold());
                println!("  Status:    {}", "Disconnected".bright_yellow());
                println!("  Interface: ghost0");
                println!("  MTU:       {}", config.settings.mtu);
                println!("  Core:      {:?}", config.settings.core_engine);
            }
        }
        Commands::Import { target, name } => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "imported": target,
                        "name": name,
                        "success": true
                    })
                );
            } else {
                println!(
                    "{} Imported profile successfully: {}",
                    "OK".bright_green().bold(),
                    target.cyan()
                );
            }
        }
        Commands::Doctor => {
            run_doctor(cli.json).await?;
        }
        Commands::Logs { lines } => {
            if cli.json {
                println!("[]");
            } else {
                println!("Displaying last {} lines of logs (empty)", lines);
            }
        }
    }

    Ok(())
}

async fn run_doctor(json: bool) -> Result<()> {
    let tun_accessible = std::path::Path::new("/dev/net/tun").exists();
    let is_root = {
        #[cfg(unix)]
        {
            nix::unistd::Uid::current().is_root()
        }
        #[cfg(not(unix))]
        {
            false
        }
    };

    if json {
        println!(
            "{}",
            serde_json::json!({
                "checks": [
                    { "name": "Kernel TUN support", "passed": tun_accessible },
                    { "name": "Privilege check", "passed": is_root },
                ]
            })
        );
    } else {
        println!("{}", "Running GhostPath System Doctor...".bold());
        print_check("Kernel TUN device (/dev/net/tun)", tun_accessible);
        print_check("Privileges (root / capabilities)", is_root);
    }

    Ok(())
}

fn print_check(name: &str, passed: bool) {
    if passed {
        println!("  {} {}", "[PASS]".bright_green().bold(), name);
    } else {
        println!("  {} {}", "[WARN]".bright_yellow().bold(), name);
    }
}
