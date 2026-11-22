use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, UnixStream};
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(
    name = "ghostpath-gui",
    about = "Desktop UI bridge and application server for GhostPath VPN"
)]
struct GuiCli {
    /// Port to serve local UI application on
    #[arg(short, long, default_value = "3001")]
    port: u16,

    /// Path to daemon Unix domain socket
    #[arg(short, long)]
    socket: Option<PathBuf>,

    /// Path to static UI dist assets directory
    #[arg(long)]
    assets_dir: Option<PathBuf>,

    /// Do not automatically open browser on launch
    #[arg(long)]
    headless: bool,
}

struct AppState {
    socket_path: PathBuf,
    assets_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = GuiCli::parse();

    let socket_path = args
        .socket
        .or_else(|| std::env::var("GHOSTPATH_SOCKET").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/run/ghostpath/ghostpathd.sock"));

    let assets_dir = args.assets_dir.unwrap_or_else(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dist = manifest_dir.join("../../UI/dist");
        if dist.exists() {
            dist
        } else {
            PathBuf::from("/usr/share/ghostpath/ui")
        }
    });

    info!(
        port = args.port,
        socket = %socket_path.display(),
        assets = %assets_dir.display(),
        "Starting GhostPath VPN GUI bridge"
    );

    let state = Arc::new(AppState {
        socket_path,
        assets_dir,
    });

    let addr = format!("127.0.0.1:{}", args.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("GhostPath VPN UI serving at http://{}", addr);

    if !args.headless {
        let url = format!("http://{}", addr);
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
            open_browser(&url);
        });
    }

    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_http_connection(&mut socket, state).await {
                        debug!("HTTP client closed: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Error accepting HTTP connection: {}", e);
            }
        }
    }
}

async fn handle_http_connection(
    stream: &mut tokio::net::TcpStream,
    state: Arc<AppState>,
) -> Result<()> {
    let mut buffer = [0u8; 8192];
    let n = stream.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    let request_str = String::from_utf8_lossy(&buffer[..n]);
    let mut lines = request_str.lines();
    let first_line = lines.next().unwrap_or_default();
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    if method == "OPTIONS" {
        let response = "HTTP/1.1 204 No Content\r\n\
                        Access-Control-Allow-Origin: *\r\n\
                        Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                        Access-Control-Allow-Headers: Content-Type\r\n\
                        Access-Control-Max-Age: 86400\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    if method == "POST" && path.starts_with("/rpc") {
        // Find body in HTTP request
        let body_start = request_str.find("\r\n\r\n").map(|idx| idx + 4).unwrap_or(0);
        let body = &request_str[body_start..];

        let rpc_res = forward_rpc_to_daemon(&state.socket_path, body).await;
        let response_body = match rpc_res {
            Ok(res) => res,
            Err(e) => serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32000,
                    "message": format!("Daemon IPC error: {}", e)
                }
            })
            .to_string(),
        };

        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Connection: close\r\n\r\n\
             {}",
            response_body.len(),
            response_body
        );

        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    // Static file serving
    let clean_path = if path == "/" || path.is_empty() {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };

    let target_file = state.assets_dir.join(clean_path);
    let final_file = if target_file.exists() && target_file.is_file() {
        target_file
    } else {
        // Fallback to index.html for SPA client-side routing
        state.assets_dir.join("index.html")
    };

    if let Ok(content) = tokio::fs::read(&final_file).await {
        let content_type = guess_mime_type(&final_file);
        let response_header = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: {}\r\n\
             Content-Length: {}\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Connection: close\r\n\r\n",
            content_type,
            content.len()
        );

        stream.write_all(response_header.as_bytes()).await?;
        stream.write_all(&content).await?;
    } else {
        let not_found = "HTTP/1.1 404 Not Found\r\n\
                         Content-Length: 9\r\n\
                         Content-Type: text/plain\r\n\r\n\
                         Not Found";
        stream.write_all(not_found.as_bytes()).await?;
    }

    Ok(())
}

async fn forward_rpc_to_daemon(socket_path: &PathBuf, body: &str) -> Result<String> {
    let unix_stream = UnixStream::connect(socket_path).await?;
    let (reader, mut writer) = unix_stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    let mut payload = body.trim().to_string();
    payload.push('\n');
    writer.write_all(payload.as_bytes()).await?;
    writer.flush().await?;

    let mut line = String::new();
    buf_reader.read_line(&mut line).await?;
    Ok(line.trim().to_string())
}

fn guess_mime_type(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn open_browser(url: &str) {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
}
