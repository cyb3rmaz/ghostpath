# Frontend Backend Communication Inventory (UI_BACKEND_INVENTORY.md)

This document provides a complete audit of every place in `UI/` that communicates with the backend, makes network requests, or generates synthetic backend data.

---

## 1. Network & IPC Call Sites in `UI/`

### 1.1 `UI/src/services/api.ts` (Primary Bridge)
- **`callRpc(method, params)`**:
  - Line 25-33: Inspects `window.__TAURI__` to invoke `rpc_request`.
  - Line 36-59: Makes HTTP POST request to `http://127.0.0.1:3001/rpc` using browser `fetch`.
  - *Action:* Completely remove the HTTP `fetch` fallback. When running in desktop mode, exclusively use Tauri IPC `invoke('rpc_request', { method, params })` or direct typed commands. Provide a clean offline error / mock mode when running in unit test environments without a window.
- **`GhostApi.connect(nodeId?)`**: Sends `connect` RPC method.
- **`GhostApi.disconnect()`**: Sends `disconnect` RPC method.
- **`GhostApi.getStatus()`**: Sends `getStatus` RPC method.
- **`GhostApi.listNodes()`**: Sends `listNodes` RPC method.
- **`GhostApi.importProfile(raw, name?)`**: Sends `importProfile` RPC method with URI or subscription.
- **`GhostApi.getSettings()`**: Sends `getSettings` RPC method.
- **`GhostApi.updateSettings(settings)`**: Sends `updateSettings` RPC method.
- **`GhostApi.getLogs(limit)`**: Sends `getLogs` RPC method.
- **`GhostApi.runDoctor()`**: Sends `runDoctor` RPC method.

---

### 1.2 `UI/src/App.tsx` (Application State & Event Loops)
- **Lines 140-161 (Mock Traffic Timer)**:
  - Runs `setInterval(..., 1000)` synthesizing upload and download speeds with `Math.random()`.
  - *Action:* Replace synthetic interval with Tauri event listener `listen('traffic_updated', (event) => ...)` receiving real byte counts from `ghostpathd`.
- **Lines 163-183 (Initial Sync on Mount)**:
  - Calls `GhostApi.getStatus()` and `GhostApi.listNodes()`.
  - *Action:* Retain initial sync via Tauri `invoke`, and subscribe to Tauri event `state_changed` and `nodes_updated` for reactive live updates.
- **Lines 185-201 (Disconnect Action)**:
  - Calls `GhostApi.disconnect()`.
  - *Action:* Await real daemon result and handle errors gracefully.
- **Lines 204-242 (Connect Action)**:
  - Calls `GhostApi.connect(activeNode.id)`.
  - Lines 225-240 contained a `setTimeout` fallback simulating a successful connection if RPC fails.
  - *Action:* Remove simulation fallback. Display real daemon errors (e.g. `DaemonNotRunning`, `PermissionDenied`, `InterfaceError`) with actionable resolution banners.
- **Lines 266-270 (`handleUpdateSettings`)**:
  - Updates local React state but does not persist to backend.
  - *Action:* Wire to `GhostApi.updateSettings(newSettings)`.

---

### 1.3 `UI/src/components/DiagnosticsView.tsx`
- **Line 4 & 47**:
  - Imports `GhostApi` and calls `await GhostApi.runDoctor()` in `handleRunLeakCheck`.
  - Currently simulates passing/failing state locally.
  - *Action:* Map real daemon doctor checks (`checks`: `tun`, `privs`, `dns`, `rp_filter`, `docker`) into the diagnostics table with real severity and fix suggestions.

---

### 1.4 `UI/src/components/AddProfileModal.tsx`
- **Line 33 & 43**:
  - Prefilled mock URL (`https://sub.baltic-mesh.io/...`) and validation check for `http://` / `https://`.
  - Currently constructs a local `ServerNode` in memory without calling `GhostApi.importProfile`.
  - *Action:* Wire modal submit to `GhostApi.importProfile(urlOrUri, friendlyName)` so profiles persist into `ghostpathd`.

---

### 1.5 `UI/src/components/TunnelView.tsx`, `SubscriptionsView.tsx`, `RoutingView.tsx`
- Currently use static seed data from `UI/src/data/vpnData.ts`.
- *Action:* Expose reactive state populated from `ghostpathd` via Tauri events and backend getters.

---

### 1.6 External Network Dependencies in `UI/index.html`
- **Lines 12-17**:
  ```html
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
  <link href="https://fonts.googleapis.com/css2?family=DM+Sans:...&family=JetBrains+Mono:...&family=Plus+Jakarta+Sans:...&display=swap" rel="stylesheet" />
  ```
  - *Violation:* Violates offline requirements and strict CSP (`connect-src 'none'; style-src 'self' 'unsafe-inline'; font-src 'self'`).
  - *Action:* Bundle fonts locally (`@fontsource` or local WOFF2 in `UI/src/assets/fonts/`), including Persian/Arabic font (Vazirmatn). Remove all external Google Fonts links.
