# GhostPath Daemon IPC & UI API Contract (API.md)

- **Version:** `1.0.0`
- **Protocol:** JSON-RPC 2.0 over Unix Domain Socket (`/run/ghostpath/ghostpathd.sock`) or Local WebSocket Bridge (`127.0.0.1:<port>`).
- **Authentication:**
  - Local Unix Socket: Kernel-enforced `SO_PEERCRED` (UID/GID verification).
  - WebSocket Bridge: Per-session cryptographically random bearer token via `Authorization: Bearer <token>` or query parameter `?token=<token>`.

---

## 1. Request / Response Format

All commands follow JSON-RPC 2.0:

### Request Frame
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "METHOD_NAME",
  "params": {}
}
```

### Success Response Frame
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {}
}
```

### Error Response Frame
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Human-readable error description",
    "data": null
  }
}
```

---

## 2. API Methods

### 2.1 Connection Control

#### `connect`
Initiates a VPN tunnel connection to the specified node or active default node.
- **Parameters:**
  ```json
  {
    "nodeId": "node-1" // optional; if omitted, connects to activeNodeId
  }
  ```
- **Returns:**
  ```json
  {
    "status": "connecting",
    "nodeId": "node-1"
  }
  ```

#### `disconnect`
Disconnects active VPN session, cleans up `ghost0`, and restores routing/DNS.
- **Parameters:** `{}`
- **Returns:**
  ```json
  {
    "status": "disconnected"
  }
  ```

#### `getStatus`
Retrieves current connection status, active node, and session metrics.
- **Parameters:** `{}`
- **Returns:**
  ```json
  {
    "status": "connected", // "disconnected" | "connecting" | "connected" | "degraded" | "failed"
    "activeNode": {
      "id": "node-1",
      "name": "Frankfurt DE-01",
      "protocol": "VLESS",
      "host": "195.181.162.88",
      "port": 443
    },
    "routingMode": "rule", // "rule" | "global" | "direct"
    "interfaceMode": "tun", // "tun" | "system-proxy"
    "traffic": {
      "downloadMbps": 48.6,
      "uploadMbps": 11.4,
      "totalDownMb": 642.4,
      "totalUpMb": 128.9,
      "sessionSeconds": 872
    }
  }
  ```

---

### 2.2 Profile & Subscription Management

#### `importProfile`
Imports a raw URI (`vless://`, `vmess://`, `trojan://`, `ss://`) or subscription URL.
- **Parameters:**
  ```json
  {
    "raw": "vless://a83e9f14-7b21-4e92-9f41-01c9b2e83c10@195.181.162.88:443?security=reality...",
    "name": "Optional Friendly Name"
  }
  ```
- **Returns:**
  ```json
  {
    "type": "single_node", // or "subscription"
    "node": {
      "id": "node-1",
      "name": "Stockholm SE-01",
      "protocol": "VLESS"
    }
  }
  ```

#### `listNodes`
Lists all available server nodes across all subscriptions and direct imports.
- **Parameters:** `{}`
- **Returns:** `ServerNode[]`

#### `pingNode`
Measures TCP/TLS hand-shake latency to a specific server node.
- **Parameters:** `{"nodeId": "node-1"}`
- **Returns:** `{"nodeId": "node-1", "latencyMs": 36}`

#### `pingAll`
Pings all nodes in parallel.
- **Parameters:** `{}`
- **Returns:** `{"results": [{"nodeId": "node-1", "latencyMs": 36}]}`

#### `listSubscriptions` / `refreshSubscription` / `deleteSubscription`
Full CRUD for subscription feeds.

---

### 2.3 Settings & Rules

#### `getSettings` / `updateSettings`
Retrieves or patches `AppSettings`:
- `coreEngine`: `"sing-box"` | `"xray-core"`
- `launchOnBoot`: boolean
- `killSwitch`: boolean
- `ipv6Routing`: boolean
- `allowLan`: boolean
- `mtu`: number (e.g. 1420)
- `socksPort`: number (10808)
- `httpPort`: number (10809)
- `dnsPrimary`: string ("1.1.1.1")
- `dnsFallback`: string ("8.8.8.8")
- `excludedInterfaces`: string[]

#### `listRules` / `addRule` / `deleteRule` / `toggleRule`
Manages split-tunnel routing rules.

---

### 2.4 Diagnostics & Doctor

#### `getLogs`
Fetches recent journal log entries filterable by level.
- **Parameters:** `{"limit": 100, "level": "all"}`
- **Returns:** `DiagnosticLog[]`

#### `runDoctor`
Executes comprehensive Linux network diagnostics.
- **Parameters:** `{}`
- **Returns:**
  ```json
  {
    "checks": [
      { "id": "tun", "name": "Kernel TUN Device", "passed": true, "fix": null },
      { "id": "privs", "name": "Daemon Capabilities", "passed": true, "fix": null },
      { "id": "dns", "name": "systemd-resolved D-Bus", "passed": true, "fix": null },
      { "id": "rp_filter", "name": "Reverse Path Filtering", "passed": true, "fix": null },
      { "id": "docker", "name": "Docker Bridge Interference", "passed": true, "fix": null }
    ]
  }
  ```

---

## 3. Streaming Server-Sent Events (WebSocket / Push)

When connected to the event stream, the server pushes notifications:

```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "type": "traffic_updated",
    "data": {
      "downloadMbps": 52.4,
      "uploadMbps": 8.1,
      "totalDownMb": 650.0,
      "totalUpMb": 130.2,
      "sessionSeconds": 873
    }
  }
}
```

Event types:
- `state_changed`: connection state transitions
- `traffic_updated`: 1-second interval bandwidth updates
- `log_appended`: realtime log streaming
- `nodes_updated`: node updates after ping or subscription refresh
- `doctor_alert`: health warnings (e.g. detected DNS leak or route flap)

---

## 4. Tauri Desktop IPC Commands & Webview Events

The desktop application (`ghostpath-gui`) eliminates the HTTP/WebSocket bridge and exposes direct native Tauri commands and event streams to the React UI:

### 4.1 Native Tauri Commands (`invoke`)

#### `rpc_request`
Passes JSON-RPC 2.0 requests directly to `ghostpathd` via Unix domain socket `/run/ghostpath/ghostpathd.sock` (with offline in-memory fallback):
- **Invocation:**
  ```typescript
  import { invoke } from '@tauri-apps/api/core';
  const result = await invoke('rpc_request', { method: 'getStatus', params: {} });
  ```

#### `check_daemon_status`
Queries host service health, checking socket existence, systemd state, permissions, and version match:
- **Invocation:**
  ```typescript
  const health = await invoke<DaemonHealth>('check_daemon_status');
  ```
- **Returns:**
  ```json
  {
    "state": "Running", // "Running" | "NotRunning" | "NotInstalled" | "PermissionDenied" | "VersionMismatch"
    "message": "GhostPath daemon is running (v0.1.0)",
    "version": "0.1.0",
    "fix_command": null
  }
  ```

### 4.2 Webview Event Streams (`listen`)

The Rust shell streams daemon notifications and desktop lifecycle events directly into the webview:

#### `state_changed`
Emitted when the VPN tunnel status changes (`disconnected`, `connecting`, `connected`, `failed`).

#### `traffic_updated`
Emitted every second during an active session with real-time download/upload rates and accumulated bytes.

#### `deep_link_imported`
Emitted when the application is launched or focused via a VPN protocol URI (`vless://`, `vmess://`, `trojan://`, `ss://`):
- **Payload:**
  ```json
  {
    "protocol": "VLESS",
    "raw": "vless://...#Frankfurt",
    "name": "Frankfurt"
  }
  ```

#### `tray_toggle_connect`
Emitted when the user clicks "Connect" / "Disconnect" in the native system tray menu.
