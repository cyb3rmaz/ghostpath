use ghostpath_daemon::engine::DaemonEngine;
use ghostpath_daemon::ipc::protocol::{JsonRpcRequest, JsonRpcResponse};
use ghostpath_daemon::ipc::IpcServer;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[tokio::test]
async fn test_daemon_ipc_server_and_client_roundtrip() {
    let socket_path = PathBuf::from(format!("/tmp/ghostpath_test_{}.sock", uuid::Uuid::new_v4()));
    let engine = Arc::new(DaemonEngine::new(true)); // dry-run mode
    let server = IpcServer::new(socket_path.clone(), engine);

    // Spawn server in background
    let server_task = tokio::spawn(async move {
        let _ = server.run().await;
    });

    // Wait for socket to become available
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect client
    let stream = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect to test Unix socket");
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    // 1. Test getStatus (initially disconnected)
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(1)),
        method: "getStatus".to_string(),
        params: None,
    };
    let mut msg = serde_json::to_string(&req).unwrap();
    msg.push('\n');
    writer.write_all(msg.as_bytes()).await.unwrap();

    let mut line = String::new();
    buf_reader.read_line(&mut line).await.unwrap();
    let resp: JsonRpcResponse = serde_json::from_str(&line).unwrap();
    assert_eq!(resp.id, Some(serde_json::json!(1)));
    let result = resp.result.expect("Expected result");
    assert_eq!(result["status"], "disconnected");

    // 2. Test connect
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(2)),
        method: "connect".to_string(),
        params: Some(serde_json::json!({})),
    };
    let mut msg = serde_json::to_string(&req).unwrap();
    msg.push('\n');
    writer.write_all(msg.as_bytes()).await.unwrap();

    line.clear();
    buf_reader.read_line(&mut line).await.unwrap();
    let resp: JsonRpcResponse = serde_json::from_str(&line).unwrap();
    assert_eq!(resp.id, Some(serde_json::json!(2)));
    let result = resp.result.expect("Expected result");
    assert_eq!(result["status"], "connected");

    // 3. Test getStatus (now connected)
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(3)),
        method: "getStatus".to_string(),
        params: None,
    };
    let mut msg = serde_json::to_string(&req).unwrap();
    msg.push('\n');
    writer.write_all(msg.as_bytes()).await.unwrap();

    line.clear();
    buf_reader.read_line(&mut line).await.unwrap();
    let resp: JsonRpcResponse = serde_json::from_str(&line).unwrap();
    assert_eq!(resp.id, Some(serde_json::json!(3)));
    let result = resp.result.expect("Expected result");
    assert_eq!(result["status"], "connected");
    assert_eq!(result["interface"], "ghost0");

    // 4. Test unknown method
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(4)),
        method: "invalidMethod".to_string(),
        params: None,
    };
    let mut msg = serde_json::to_string(&req).unwrap();
    msg.push('\n');
    writer.write_all(msg.as_bytes()).await.unwrap();

    line.clear();
    buf_reader.read_line(&mut line).await.unwrap();
    let resp: JsonRpcResponse = serde_json::from_str(&line).unwrap();
    assert_eq!(resp.id, Some(serde_json::json!(4)));
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32601);

    // 5. Test disconnect
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::json!(5)),
        method: "disconnect".to_string(),
        params: None,
    };
    let mut msg = serde_json::to_string(&req).unwrap();
    msg.push('\n');
    writer.write_all(msg.as_bytes()).await.unwrap();

    line.clear();
    buf_reader.read_line(&mut line).await.unwrap();
    let resp: JsonRpcResponse = serde_json::from_str(&line).unwrap();
    assert_eq!(resp.id, Some(serde_json::json!(5)));
    let result = resp.result.expect("Expected result");
    assert_eq!(result["status"], "disconnected");

    // Cleanup
    server_task.abort();
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
}
