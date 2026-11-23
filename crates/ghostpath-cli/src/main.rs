use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod client;
use client::DaemonClient;

#[derive(Parser, Debug)]
#[command(
    name = "ghostpath",
    about = "GhostPath VPN - Linux VPN client with reliable TUN mode"
)]
struct Cli {
    /// Format output as JSON for scripting
    #[arg(long, global = true)]
    json: bool,

    /// Path to custom daemon Unix domain socket
    #[arg(long, global = true)]
    socket: Option<PathBuf>,

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
    /// List all imported VPN profiles
    Profiles,
    /// Import a server profile link (vless://, vmess://, vpn://, etc.) or config file
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
    let socket_path = cli.socket.unwrap_or_else(DaemonClient::default_socket_path);
    let client = DaemonClient::new(socket_path);

    match cli.command {
        Commands::Connect { node_id } => {
            let params = serde_json::json!({
                "nodeId": node_id,
            });

            if !cli.json {
                println!("{} Connecting to VPN node...", "->".bright_green().bold());
            }

            match client.call("connect", params).await {
                Ok(res) => {
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&res)?);
                    } else {
                        let node_name = res
                            .get("nodeName")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Connected");
                        let iface = res
                            .get("interface")
                            .and_then(|v| v.as_str())
                            .unwrap_or("ghost0");
                        println!(
                            "{} Tunnel established to {} via {}",
                            "OK".bright_green().bold(),
                            node_name.cyan(),
                            iface.yellow()
                        );
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({ "error": e.to_string() }));
                    } else {
                        eprintln!("{} Failed to connect: {}", "ERR".bright_red().bold(), e);
                    }
                    std::process::exit(1);
                }
            }
        }
        Commands::Disconnect => {
            if !cli.json {
                println!("{} Tearing down tunnel...", "->".bright_yellow().bold());
            }

            match client.call("disconnect", serde_json::json!({})).await {
                Ok(res) => {
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&res)?);
                    } else {
                        println!(
                            "{} Tunnel disconnected and routes cleanly restored",
                            "OK".bright_green().bold()
                        );
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({ "error": e.to_string() }));
                    } else {
                        eprintln!("{} Failed to disconnect: {}", "ERR".bright_red().bold(), e);
                    }
                    std::process::exit(1);
                }
            }
        }
        Commands::Status => match client.call("getStatus", serde_json::json!({})).await {
            Ok(res) => {
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    let status_str = res
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let iface = res
                        .get("interface")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ghost0");
                    let mtu = res.get("mtu").and_then(|v| v.as_u64()).unwrap_or(1420);

                    println!("{}", "GhostPath VPN Status:".bold());
                    let colored_status = match status_str {
                        "connected" => status_str.bright_green().bold(),
                        "connecting" => status_str.bright_yellow(),
                        "disconnecting" => status_str.bright_yellow(),
                        _ => status_str.bright_red(),
                    };
                    println!("  Status:    {}", colored_status);
                    println!("  Interface: {}", iface);
                    println!("  MTU:       {}", mtu);

                    if let Some(node) = res.get("activeNode").filter(|v| !v.is_null()) {
                        let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                        let proto = node.get("protocol").and_then(|v| v.as_str()).unwrap_or("-");
                        let host = node.get("host").and_then(|v| v.as_str()).unwrap_or("-");
                        let port = node.get("port").and_then(|v| v.as_u64()).unwrap_or(0);
                        println!(
                            "  Server:    {} ({}://{}:{})",
                            name.cyan(),
                            proto,
                            host,
                            port
                        );
                    }

                    if let Some(traffic) = res.get("traffic") {
                        let down_mbps = traffic
                            .get("downloadMbps")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        let up_mbps = traffic
                            .get("uploadMbps")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        let secs = traffic
                            .get("sessionSeconds")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        println!(
                            "  Speed:     ↓ {:.1} Mbps  ↑ {:.1} Mbps",
                            down_mbps, up_mbps
                        );
                        println!("  Uptime:    {}s", secs);
                    }
                }
            }
            Err(e) => {
                if cli.json {
                    println!("{}", serde_json::json!({ "error": e.to_string() }));
                } else {
                    eprintln!("{} Failed to get status: {}", "ERR".bright_red().bold(), e);
                }
                std::process::exit(1);
            }
        }
        Commands::Profiles => match client.call("listNodes", serde_json::json!({})).await {
            Ok(res) => {
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else if let Some(nodes) = res.as_array() {
                    if nodes.is_empty() {
                        println!("No profiles found. Import one using 'ghostpath import <link | file>'.");
                    } else {
                        println!("{}", "Imported VPN Profiles:".bold());
                        for node in nodes {
                            let id = node.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                            let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                            let proto = node.get("protocol").and_then(|v| v.as_str()).unwrap_or("-");
                            let endpoint = format!(
                                "{}:{}",
                                node.get("host").and_then(|v| v.as_str()).unwrap_or("-"),
                                node.get("port").and_then(|v| v.as_u64()).unwrap_or(0)
                            );
                            println!(
                                "  • [{}] {} ({}) - {}",
                                proto.yellow().bold(),
                                name.cyan(),
                                endpoint,
                                id.dimmed()
                            );
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{} Failed to list profiles: {}", "ERR".bright_red().bold(), e);
                std::process::exit(1);
            }
        },
        Commands::Import { target, name } => {
            let content = if std::path::Path::new(&target).is_file() {
                match std::fs::read_to_string(&target) {
                    Ok(text) => text,
                    Err(e) => {
                        eprintln!(
                            "{} Failed to read file '{}': {}",
                            "ERR".bright_red().bold(),
                            target,
                            e
                        );
                        std::process::exit(1);
                    }
                }
            } else {
                target.clone()
            };

            let params = serde_json::json!({
                "raw": content,
                "name": name,
            });

            match client.call("importProfile", params).await {
                Ok(res) => {
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&res)?);
                    } else if res.get("type").and_then(|v| v.as_str()) == Some("amnezia_multi") {
                        let count = res.get("importedCount").and_then(|v| v.as_u64()).unwrap_or(0);
                        println!(
                            "{} Imported {} Amnezia profile(s) successfully.",
                            "OK".bright_green().bold(),
                            count
                        );
                        if let Some(unsupported) = res.get("unsupportedContainers").and_then(|v| v.as_array()) {
                            for item in unsupported {
                                let c_name = item.get("container").and_then(|v| v.as_str()).unwrap_or("unknown");
                                let reason = item.get("reason").and_then(|v| v.as_str()).unwrap_or("unsupported");
                                println!(
                                    "  {} Skipped unsupported container '{}': {}",
                                    "WARN".bright_yellow().bold(),
                                    c_name.cyan(),
                                    reason
                                );
                            }
                        }
                    } else {
                        let node_name = res
                            .get("node")
                            .and_then(|n| n.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or(&target);
                        println!(
                            "{} Imported profile successfully: {}",
                            "OK".bright_green().bold(),
                            node_name.cyan()
                        );
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({ "error": e.to_string() }));
                    } else {
                        eprintln!(
                            "{} Failed to import profile: {}",
                            "ERR".bright_red().bold(),
                            e
                        );
                    }
                    std::process::exit(1);
                }
            }
        }
        Commands::Doctor => match client.call("runDoctor", serde_json::json!({})).await {
            Ok(res) => {
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    println!("{}", "GhostPath System Diagnostics:".bold());
                    if let Some(checks) = res.get("checks").and_then(|v| v.as_array()) {
                        for check in checks {
                            let name = check.get("name").and_then(|v| v.as_str()).unwrap_or("-");
                            let passed = check
                                .get("passed")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            if passed {
                                println!("  {} {}", "[PASS]".bright_green().bold(), name);
                            } else {
                                println!("  {} {}", "[WARN]".bright_yellow().bold(), name);
                                if let Some(fix) = check.get("fix").and_then(|v| v.as_str()) {
                                    println!("         Fix: {}", fix.dimmed());
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if cli.json {
                    println!("{}", serde_json::json!({ "error": e.to_string() }));
                } else {
                    eprintln!("{} Failed to run doctor: {}", "ERR".bright_red().bold(), e);
                }
                std::process::exit(1);
            }
        },
        Commands::Logs { lines } => {
            let params = serde_json::json!({
                "limit": lines,
            });

            match client.call("getLogs", params).await {
                Ok(res) => {
                    if cli.json {
                        println!("{}", serde_json::to_string_pretty(&res)?);
                    } else if let Some(logs) = res.as_array() {
                        if logs.is_empty() {
                            println!("(No log entries recorded)");
                        } else {
                            for log in logs {
                                if let Some(line) = log.as_str() {
                                    println!("{}", line);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    if cli.json {
                        println!("{}", serde_json::json!({ "error": e.to_string() }));
                    } else {
                        eprintln!(
                            "{} Failed to retrieve logs: {}",
                            "ERR".bright_red().bold(),
                            e
                        );
                    }
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}
