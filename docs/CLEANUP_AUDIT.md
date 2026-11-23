# GhostPath VPN - Fake, Demo, and Non-Functional Content Audit

**Audit Date:** 2026-11-24  
**Audit Status:** Complete — Pending User Review (No deletions performed prior to approval)  
**Standard:** Strict zero-mock, honest empty states, and verified production code paths.

---

## 1. Executive Summary

This comprehensive audit inspected the entire GhostPath VPN repository across the React frontend (`UI/`), native Tauri shell (`crates/ghostpath-gui`), system daemon (`crates/ghostpath-daemon`), CLI client (`crates/ghostpath-cli`), core crates (`ghostpath-core`, `ghostpath-net`, `ghostpath-proto`, `ghostpath-tun`), packaging scripts, and documentation.

### Core Findings:
1. **Frontend Hard-Coded Demo Content (`UI/src/data/vpnData.ts` & `App.tsx`):**
   - 9 fake server nodes with hardcoded European/US/Asian IPs and latencies.
   - 2 fake remote subscriptions with made-up tokenized URLs.
   - 6 hardcoded mock diagnostic log entries.
   - Synthetic sine/cosine waves for throughput history.
   - Initial application state defaulted to `connected` with fabricated session durations and bandwidth counters (872s, 48.6 Mbps down, 11.4 Mbps up, 642 MB total).
2. **Simulation & Random Generators in UI (`UI/src/App.tsx` & `AddProfileModal.tsx`):**
   - `setInterval` simulating bandwidth traffic with `Math.random()` (even though real daemon push events exist).
   - `handlePingNode` and `handlePingAll` using `setTimeout` and `Math.random()` to generate fake latency numbers.
   - `handleRefreshSubscription` using `setTimeout` to pretend to sync remote endpoints.
   - "Paste Sample URI" and "Paste Sample URL" buttons in `AddProfileModal.tsx`.
   - Subscription import generating fake Tallinn and Riga server nodes in memory instead of fetching feeds.
3. **Mock Backend Code Reachable in Production:**
   - `crates/ghostpath-daemon/src/mock.rs` and the `--mock` CLI flag compiled directly into `ghostpathd`.
   - `crates/ghostpath-gui/src/ipc/mock.rs` compiled into release builds with an automatic silent fallback in `handle_rpc_invoke` when the daemon is offline.
   - `UI/src/services/api.ts` running an in-memory mock fallback (`emulateOfflineFallback`) if desktop IPC fails.
4. **Hardcoded Fallbacks & Stubs in Rust Daemon (`crates/ghostpath-daemon/src/engine.rs`):**
   - Hardcoded fallback server node (`"Frankfurt Reality-01"`, `195.181.162.88`) when connecting without imported profiles.
   - Hardcoded throughput numbers (`48.6` Mbps down, `11.4` Mbps up) in `get_status()`.
   - Unhandled settings fields (`coreEngine`, `dnsPrimary`, `dnsFallback`, `socksPort`, `tunStack`) in `update_settings()`.
5. **Dead Scaffolding & Unused Dependencies (`UI/package.json`, AI Studio leftovers):**
   - Unused dependencies: `@google/genai`, `express`, `@types/express`, `dotenv`, `motion`.
   - AI Studio template files: `UI/.env.example`, `UI/metadata.json`, `UI/README.md`.
   - Placeholder screenshot in AppStream metadata (`packaging/ghostpath.metainfo.xml`).

---

## 2. Comprehensive Inventory & Action Matrix

### Classification Legend:
- **1. Remove:** Pure decoration, demo buttons, fake timers, or dead assets with no real counterpart.
- **2. Replace with real data:** A real source exists or should exist; wire the component to the real daemon/IPC.
- **3. Replace with an empty state:** Honest zero-state when data is absent (e.g. no profiles, 0 bytes, not connected).
- **4. Keep in tests only:** Move mocks and fixtures into `#[cfg(test)]` or a test-only `dev-mock` Cargo feature.
- **5. Ask me:** Open questions requiring design decisions.

| ID | File | Line(s) | Item Description | Classification | Planned Action & Real Code Path |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **UI-01** | `UI/src/data/vpnData.ts` | 10–31 | `INITIAL_SUBSCRIPTIONS` with fake URLs (`sub.nordic-relay.net`, `vault.mazyar-ops.org`) | **3. Replace with empty state** | Set to `[]`. UI renders honest empty state: *"No active subscriptions. Import a subscription URL to populate server nodes."* |
| **UI-02** | `UI/src/data/vpnData.ts` | 33–169 | `INITIAL_NODES` (9 fake servers across DE, NL, FI, CH, GB, US, JP, FR) | **3. Replace with empty state** | Set to `[]`. Tunnel view renders honest empty state: *"No server profiles imported. Import a VLESS, VMess, Trojan, or subscription link."* |
| **UI-03** | `UI/src/data/vpnData.ts` | 171–236 | `INITIAL_RULES` (8 hardcoded routing rules) | **2. Replace with real data** | Fetch initial rules from `GhostApi.listRules()` backed by `ghostpath_core::rules::default_routing_rules()`. |
| **UI-04** | `UI/src/data/vpnData.ts` | 238–281 | `INITIAL_LOGS` (6 hardcoded fake logs) | **3. Replace with empty state** | Set to `[]`. Real logs load exclusively from `GhostApi.getLogs()` and daemon journal events. |
| **UI-05** | `UI/src/data/vpnData.ts` | 283–287 | `INITIAL_TRAFFIC_HISTORY` (24 pre-allocated points) | **3. Replace with empty state** | Initialize to 24 points with `0` Mbps. Flatline baseline until real traffic flows. |
| **UI-06** | `UI/src/data/vpnData.ts` | 289–301 | `INITIAL_SETTINGS` | **2. Replace with real data** | Load actual persisted settings from `GhostApi.getSettings()`. |
| **UI-07** | `UI/src/App.tsx` | 76 | `status` initialized to `'connected'` | **3. Replace with empty state** | Initialize `status` to `'disconnected'`. App starts honest and disconnected. |
| **UI-08** | `UI/src/App.tsx` | 80–82 | State initialized with `INITIAL_SUBSCRIPTIONS`, `INITIAL_NODES`, `INITIAL_NODES[0].id` | **3. Replace with empty state** | Initialize to `subscriptions = []`, `nodes = []`, `activeNodeId = ''`. Handle null active node safely. |
| **UI-09** | `UI/src/App.tsx` | 90–94 | Hardcoded counters: 872s, 48.6 Mbps down, 11.4 Mbps up, 642.4 MB total down, 128.9 MB total up | **3. Replace with empty state** | Initialize all metrics strictly to `0`. Populate only from real `onTrafficUpdated` daemon events. |
| **UI-10** | `UI/src/App.tsx` | 95–101 | Traffic history initialized with `Math.sin` and `Math.cos` wave | **1. Remove** | Replace with clean zero-throughput array `Array(24).fill({ downloadMbps: 0, uploadMbps: 0 })`. |
| **UI-11** | `UI/src/App.tsx` | 137–167 | `useEffect` with `setInterval` simulating throughput using `Math.random()` | **1. Remove** | Delete simulated timer completely. Rely 100% on real `GhostApi.onTrafficUpdated` event stream from `ghostpathd`. |
| **UI-12** | `UI/src/App.tsx` | 305–323 | `handlePingNode` simulating latency with `setTimeout` and `Math.random()` | **2. Replace with real data** | Wire to real daemon IPC `pingNode(nodeId)` which executes a real TCP handshake latency check. |
| **UI-13** | `UI/src/App.tsx` | 325–351 | `handlePingAll` simulating sweep with `setTimeout` and `Math.random()` | **2. Replace with real data** | Wire to real daemon IPC `pingAll()` which performs parallel TCP connection probes. |
| **UI-14** | `UI/src/App.tsx` | 385–408 | `handleRefreshSubscription` and `handleRefreshAllSubs` simulating sync with `setTimeout` | **2. Replace with real data** | Wire to real daemon IPC `refreshSubscription(id)` which re-fetches the URL and updates `config.nodes`. |
| **UI-15** | `UI/src/App.tsx` | 420–444 | Split-tunnel rules state (`rules`) held purely in React local memory | **2. Replace with real data** | Wire `toggleRule`, `changeRuleAction`, `deleteRule`, `addRule` to daemon IPC (`addRule`, `deleteRule`, `updateRule`). |
| **UI-16** | `UI/src/components/TunnelView.tsx` | 158–163 | Assumes `activeNode` is always present (`activeNode.name`, `activeNode.protocol`) | **3. Replace with empty state** | Add null guard: when `nodes.length === 0`, render empty state banner with prominent "Import Profile" button. |
| **UI-17** | `UI/src/components/TunnelView.tsx` | 308–312 | Exit IPv4 displays `activeNode.host` or hardcoded `'Local ISP'` | **2. Replace with real data** | Show `activeNode?.host || 'Disconnected'`. |
| **UI-18** | `UI/src/components/DiagnosticsView.tsx` | 88 | Hardcoded fake IP: `'84.17.52.109 (ISP)'` | **3. Replace with empty state** | Replace with `"Not Connected"` when tunnel is inactive. |
| **UI-19** | `UI/src/components/DiagnosticsView.tsx` | 98 | Hardcoded text `"AAAA Suppressed"` regardless of config | **2. Replace with real data** | Dynamically display based on `settings.ipv6Routing ? 'Dual-Stack Enabled' : 'AAAA Blocked'`. |
| **UI-20** | `UI/src/components/AddProfileModal.tsx` | 26–35 | `handleInsertSampleUri` and `handleInsertSampleSub` inserting fake demo endpoints | **1. Remove** | Delete sample insertion methods. |
| **UI-21** | `UI/src/components/AddProfileModal.tsx` | 225–230 | "Paste Sample URL" button | **1. Remove** | Replace with functional "Paste from Clipboard" using `navigator.clipboard.readText()`. |
| **UI-22** | `UI/src/components/AddProfileModal.tsx` | 250–255 | "Paste Sample URI" button | **1. Remove** | Replace with functional "Paste from Clipboard" using `navigator.clipboard.readText()`. |
| **UI-23** | `UI/src/components/AddProfileModal.tsx` | 59–90 | Generates fake Tallinn and Riga nodes upon subscription import | **2. Replace with real data** | Wire to real daemon IPC `importSubscription(url, name)` which fetches and parses nodes using `ghostpath-proto`. |
| **UI-24** | `UI/src/components/AddProfileModal.tsx` | 116–133 | Hardcoded fallback host `195.181.162.88` and `sub-personal` in client-side URI parser | **2. Replace with real data** | Send raw URI directly to daemon `importProfile(rawUri)` which parses it accurately via `parse_any_link`. |
| **UI-25** | `UI/src/services/api.ts` | 59–153 | `emulateOfflineFallback` returning mock nodes, mock traffic, mock doctor checks | **1. Remove / 4. Keep in tests only** | Remove from production bundle. If Tauri IPC is unavailable, reject with a clear error: `"Desktop IPC bridge unavailable"`. |
| **UI-26** | `UI/src/services/api.ts` | 239–244 | `checkDaemonStatus` returns `{ state: 'running' }` fallback when Tauri is not detected | **3. Replace with empty state** | Return `{ state: 'not_running', message: 'Desktop IPC is unavailable' }`. |
| **UI-27** | `UI/package.json` | 18, 21–22, 24, 30, 37 | Dead dependencies: `@google/genai`, `express`, `@types/express`, `dotenv`, `motion`, `tsx` | **1. Remove** | Delete all unused dependencies from `package.json` and run `npm install` to clean `package-lock.json`. |
| **UI-28** | `UI/package.json` | 2, 4 | Scaffolding name `"react-example"` and version `"0.0.0"` | **2. Replace with real data** | Update to `"name": "ghostpath-ui"`, `"version": "0.1.0"`. |
| **UI-29** | `UI/.env.example` | 1–9 | AI Studio Gemini API template (`GEMINI_API_KEY`, `APP_URL`) | **1. Remove** | Delete file completely. |
| **UI-30** | `UI/metadata.json` | 1–6 | AI Studio metadata referencing `"Kestrel VPN"` and `MAJOR_CAPABILITY_SERVER_SIDE_GEMINI_API` | **1. Remove** | Delete file completely. |
| **UI-31** | `UI/README.md` | 1–20 | AI Studio template readme with remote image banner and instructions to get Gemini API keys | **1. Remove** | Delete file completely or replace with GhostPath UI build documentation. |
| **UI-32** | `UI/vite.config.ts` | 12–29, 37–42 | Copying non-existent `src/assets` and AI Studio HMR comments | **1. Remove** | Clean up `vite.config.ts` to standard, clean Vite config. |
| **DAEMON-01** | `crates/ghostpath-daemon/src/mock.rs` | 1–360 | `MockDaemonServer` (mock TCP server with fake Stockholm/Frankfurt nodes and fake logs) | **1. Remove** | Delete `mock.rs` or move completely to test fixtures. |
| **DAEMON-02** | `crates/ghostpath-daemon/src/main.rs` | 33–40, 55–60 | `--mock` and `--port` CLI flags in `ghostpathd` | **1. Remove** | Remove `--mock` flag from daemon binary. |
| **DAEMON-03** | `crates/ghostpath-daemon/src/engine.rs` | 91–110 | Default fallback to fake `"Frankfurt Reality-01"` (`195.181.162.88`) when connecting without profiles | **2. Replace with real data / error** | Return error: `bail!("No server profiles available. Please import a profile first.")`. |
| **DAEMON-04** | `crates/ghostpath-daemon/src/engine.rs` | 304–305 | Hardcoded `48.6` Mbps down and `11.4` Mbps up in `get_status()` | **2. Replace with real data** | Calculate real rates dynamically from differential `bytes_in`/`bytes_out` over elapsed time. |
| **DAEMON-05** | `crates/ghostpath-daemon/src/engine.rs` | 358–374 | `update_settings()` ignores `coreEngine`, `dnsPrimary`, `dnsFallback`, `socksPort`, `tunStack` | **2. Replace with real data** | Persist all fields in `GhostConfig.settings` so user preference edits actually apply. |
| **DAEMON-06** | `crates/ghostpath-daemon/src/ipc/handler.rs` | 1–95 | Missing IPC commands for pinging, subscriptions, and rules | **2. Replace with real data** | Implement `pingNode`, `pingAll`, `listSubscriptions`, `refreshSubscription`, `deleteSubscription`, `listRules`, `addRule`, `deleteRule`, `toggleRule`. |
| **GUI-01** | `crates/ghostpath-gui/src/ipc/mock.rs` | 1–222 | `MockDaemon` in production crate | **4. Keep in tests only** | Gate under `#[cfg(any(test, feature = "dev-mock"))]`. Release builds will not compile it. |
| **GUI-02** | `crates/ghostpath-gui/src/ipc/commands.rs` | 44–58 | Automatic silent fallback to `mock_daemon.handle_rpc()` when `ipc_client` is unavailable | **2. Replace with real data / error** | In release builds, return `Err("GhostPath daemon is not running. Please start ghostpathd service.")`. Mock only allowed under test / `dev-mock`. |
| **GUI-03** | `crates/ghostpath-gui/src/ipc/commands.rs` | 56–58 | `handle_check_daemon_status` returns fake `DaemonHealth::running` when `force_mock` is set | **4. Keep in tests only** | Only allow `force_mock` under `#[cfg(any(test, feature = "dev-mock"))]`. |
| **PKG-01** | `packaging/ghostpath.metainfo.xml` | 37–41 | Remote placeholder screenshot URL `https://raw.githubusercontent.com/.../dashboard.png` | **1. Remove** | Remove non-existent `<screenshot>` block until a real verified screenshot is taken and published. |

---

## 3. Real Implementation Plan for Categories 2 & 3

### Category 2 (Wired to Real Daemon Sources):
1. **Real Latency Ping (`pingNode`, `pingAll`):**
   - Implement real TCP handshake latency probing in `crates/ghostpath-daemon/src/engine.rs` using `tokio::time::timeout` and `tokio::net::TcpStream::connect((host, port))`.
   - Wire `GhostApi.pingNode` and `GhostApi.pingAll` to these methods.
2. **Real Subscription Sync (`listSubscriptions`, `refreshSubscription`, `deleteSubscription`):**
   - In `DaemonEngine`, implement `refresh_subscription(id)`: fetch subscription URL (via `reqwest`/`ureq` or tunnel proxy), parse content using `ghostpath_proto::parser::subscription::parse_subscription_content`, and update `config.nodes`.
   - Wire `SubscriptionsView` sync buttons directly to this command.
3. **Real Split-Tunnel Rules (`listRules`, `addRule`, `deleteRule`, `toggleRule`):**
   - In `DaemonEngine`, store and persist `RuleEngine` in `GhostConfig`.
   - Initialize rules from `default_routing_rules()`.
   - Wire `RoutingView` directly to IPC methods so rule toggles and additions persist to `~/.config/ghostpath/config.toml`.
4. **Real Throughput Counters:**
   - In `DaemonEngine`, track `bytes_in` and `bytes_out` on `ghost0` / proxy outbound.
   - Calculate real `download_mbps = (delta_bytes_in * 8) / (delta_time * 1_000_000)`.
   - Emit real metrics in `traffic_updated` push events.

### Category 3 (Honest Empty States):
1. **Fresh Install Profile View:**
   - When `nodes.length === 0`:
     - Big tactual card: "No Server Profiles"
     - Subtitle: "Import a VLESS, VMess, Trojan, or subscription share link to get started."
     - Action button: `[+ Import Profile]` (opens modal).
     - Connect toggle disabled with clear tooltip: "Import a server profile first".
2. **Fresh Install Subscriptions View:**
   - When `subscriptions.length === 0`:
     - Card: "No Active Subscriptions"
     - Action: `[+ Add Subscription]`.
3. **Fresh Install Logs View:**
   - When `logs.length === 0`:
     - "No journal entries yet. Logs appear here when the daemon runs or initiates a connection."
4. **Fresh Install Telemetry:**
   - 0.0 Mbps down, 0.0 Mbps up, 0 B session RX/TX, 00:00:00 duration.
   - Flatline chart baseline.

---

## 4. Test Isolation (Category 4)

1. Move `crates/ghostpath-gui/src/ipc/mock.rs` under feature flag:
   ```toml
   [features]
   default = []
   dev-mock = []
   ```
   ```rust
   #[cfg(any(test, feature = "dev-mock"))]
   pub mod mock;
   ```
2. In `crates/ghostpath-gui/tests/gui_e2e_tests.rs`:
   - Runs under `#[cfg(test)]`, maintaining 100% test coverage while completely excluding `mock.rs` from release artifacts.
3. Add an automated CI check scan script:
   - `scripts/verify_no_demo_content.sh`: scans release binaries and `UI/dist` for forbidden substrings (`Math.random`, `lorem`, `fake`, `sample-nordic`, `mock-node`, `INITIAL_NODES`, `baltic-mesh`).

---

## 5. Review & Confirmation Request

Per `.agents/rules/superpowers.md`, no code deletions or modifications occurred until this audit was reviewed and approved.
The user formally reviewed and approved execution.

---

## 6. Execution & Verification Log

All 38 classified audit items have been executed and verified in small, focused commits:

1. **Commit `3b9f493` - Area 1: Assets & Scaffolding Cleanup:**
   - Deleted unused npm dependencies (`@google/genai`, `express`, `@types/express`, `dotenv`, `motion`, `tsx`).
   - Renamed package to `"ghostpath-ui"` v0.1.0 in `UI/package.json`.
   - Purged AI Studio debris: `UI/.env.example`, `UI/metadata.json`, `UI/README.md`.
   - Cleaned `UI/vite.config.ts` and removed non-existent remote screenshot in `packaging/ghostpath.metainfo.xml`.

2. **Commit `6c11592` - Area 2: Daemon & Core Real Data Wiring:**
   - Deleted `crates/ghostpath-daemon/src/mock.rs` and removed `--mock` and `--port` CLI options.
   - Removed fake fallback server node in `engine.rs`; connect with no profiles returns clear error.
   - Implemented real differential bandwidth rate tracking based on atomic network counters.
   - Implemented real IPC methods: `ping_node`, `ping_all`, `list_subscriptions`, `import_subscription`, `delete_subscription`, `refresh_subscription`, `list_rules`, `add_rule`, `delete_rule`, `toggle_rule`.
   - Verified with 8 passing unit tests in `ghostpath-daemon`.

3. **Commit `c27eebb` - Area 3: GUI Mock Isolation:**
   - Added `dev-mock = []` Cargo feature to `ghostpath-gui`.
   - Gated `MockDaemon` strictly behind `#[cfg(any(test, feature = "dev-mock"))]`.
   - In production builds, offline daemon returns clear error: `"GhostPath daemon is not running. Please start the ghostpathd service."` with zero silent mock fallback.
   - Verified that `strings target/release/deps/libghostpath_gui-*.rlib | grep MockDaemon` returns zero matches.

4. **Commit `78abbd1` - Area 4: UI Honest Empty States & Real Data Wiring:**
   - Emptied `INITIAL_SUBSCRIPTIONS = []`, `INITIAL_NODES = []`, `INITIAL_RULES = []`, `INITIAL_LOGS = []` in `UI/src/data/vpnData.ts`.
   - Set honest zero-state defaults in `App.tsx`: `status: 'disconnected'`, speeds `0.0`, session duration `0`.
   - Purged fake `setInterval` random throughput generator loop.
   - Purged `emulateOfflineFallback` from `UI/src/services/api.ts`.
   - Replaced "Paste Sample" buttons in `AddProfileModal.tsx` with functional "Paste from Clipboard".
   - Connected all UI views to real daemon IPC calls.
   - Added null-safe guards for active node display.

5. **Area 5: Verification Scanner & CI Integration:**
   - Created `scripts/verify_no_demo_content.sh` scanning `UI/src`, `UI/dist`, and compiled release libraries.
   - Added verification scanner as an enforcement step in `.github/workflows/desktop_ci.yml`.
   - Updated `docs/BACKLOG.md` with deferred enhancements.
