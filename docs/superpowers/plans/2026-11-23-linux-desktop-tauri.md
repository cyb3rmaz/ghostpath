# Native Linux Desktop App (Tauri 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the GhostPath VPN web UI (`UI/`) into a native, offline Linux desktop application (`ghostpath-gui`) built with Tauri 2 and WebKitGTK, communicating with `ghostpathd` over Unix domain socket IPC with streaming events, system tray, single instance, deep link handlers, strict CSP, and full Linux packaging (.deb, .rpm, AppImage, PKGBUILD).

**Architecture:**
A native Rust desktop shell (`crates/ghostpath-gui`) powered by Tauri 2 embeds the compiled React frontend from `UI/dist`. All local HTTP server and external browser dependencies are eliminated. A Unix domain socket client in Rust connects to `/run/ghostpath/ghostpathd.sock`, exposes typed Tauri `invoke` commands (`rpc_request`, `check_daemon_status`), and pushes real-time daemon events (`traffic_updated`, `state_changed`, `log_appended`) via Tauri window events. A thin adapter in `UI/src/services/api.ts` transparently bridges UI calls to Tauri without changing UI presentation.

**Tech Stack:**
- **Rust / Desktop Shell:** Tauri 2 (`tauri`, `tauri-build`), `tauri-plugin-single-instance`, `tauri-plugin-notification`, `tauri-plugin-deep-link`, `tokio` (Unix domain sockets), `tracing`, `serde`, `serde_json`
- **Frontend:** React 19, TypeScript 5.8+, Vite 8, Tailwind CSS v4, Lucide React, Motion
- **Target OS:** Linux (Ubuntu 22.04/24.04 Wayland/GNOME, Fedora, Arch KDE/X11)
- **Packaging:** Debian `.deb`, Red Hat `.rpm`, AppImage, Arch `PKGBUILD`

**Spec:**
- User Request: Turning GhostPath VPN web UI into a native Linux desktop app
- `docs/DECISIONS.md` (Section 3: Tauri 2 Decision)
- `docs/API.md` (Daemon IPC JSON-RPC 2.0 Contract)
- `docs/UI_BACKEND_INVENTORY.md` (Frontend Call Inventory)

## Global Constraints

- Runs as standard non-root user (authorized via `ghostpath` group or polkit).
- Zero remote network requests from UI webview (`connect-src 'none'`). All fonts and assets bundled locally.
- Strict Tauri capabilities and CSP; devtools disabled in release builds.
- Never log secrets (UUIDs, Reality keys, passwords, subscription URLs).
- Clean daemon state handling (NotInstalled, NotRunning, PermissionDenied, VersionMismatch) with actionable UI banners.

## Review Focus

1. **Daemon Socket Unavailability / Permission Denied:** UI must render a clean banner with "Start Service" or permission instructions, never a blank screen or hanging promise.
2. **Offline Font & Asset Bundling:** With network interfaces down, webview must render DM Sans, JetBrains Mono, Plus Jakarta Sans, and Persian Vazirmatn font without visual degradation or CSP console errors.
3. **Single Instance Activation:** Launching a second instance with or without arguments (`vless://...`) must focus the existing window and send the deep link payload to the running instance.
4. **Window Hide to Tray:** Closing window hides it to system tray; quit menu item cleanly exits the app and closes background event streaming tasks.
5. **No Route / Rule Leaks on Package Uninstall:** Package pre-remove/post-remove scripts must tear down `ghost0`, routes, and nftables rules cleanly.

---

### Task 1: Complete Phase 1 - Plan, Decision Record & UI Inventory

**Files:**
- Modify: `docs/DECISIONS.md`
- Create: `docs/UI_BACKEND_INVENTORY.md`
- Create: `docs/superpowers/plans/2026-11-23-linux-desktop-tauri.md`

**Interfaces:**
- Consumes: `docs/API.md`, `UI/src/services/api.ts`, `UI/src/App.tsx`
- Produces: Confirmed architectural decisions and audit of all frontend-backend touchpoints

- [ ] **Step 1: Write UI inventory and decision record**
Audit every call in `UI/` and document replacement strategies in `docs/UI_BACKEND_INVENTORY.md` and `docs/DECISIONS.md`.

- [ ] **Step 2: Verify documentation consistency**
Ensure all 8 phases are captured with clear verification criteria.

- [ ] **Step 3: Commit Phase 1 deliverable**
```bash
git add docs/DECISIONS.md docs/UI_BACKEND_INVENTORY.md docs/superpowers/plans/2026-11-23-linux-desktop-tauri.md
git commit -m "docs: complete Phase 1 design record, plan, and UI backend inventory"
```

---

### Task 2: Phase 2 - Tauri 2 Shell & Offline Asset Bundling (Mock Backend)

**Files:**
- Modify: `crates/ghostpath-gui/Cargo.toml`
- Create: `crates/ghostpath-gui/build.rs`
- Create: `crates/ghostpath-gui/tauri.conf.json`
- Create: `crates/ghostpath-gui/capabilities/default.json`
- Create: `crates/ghostpath-gui/src/ipc/mock.rs`
- Create: `crates/ghostpath-gui/src/ipc/commands.rs`
- Modify: `crates/ghostpath-gui/src/lib.rs`
- Modify: `crates/ghostpath-gui/src/main.rs`
- Modify: `UI/index.html`
- Modify: `UI/src/index.css`
- Create: `UI/src/assets/fonts/` (bundled font files)

**Interfaces:**
- Consumes: `UI/dist`
- Produces: `ghostpath_gui::run()` Tauri application with mock RPC support and offline fonts

- [ ] **Step 1: Write failing unit test for mock RPC handler**
In `crates/ghostpath-gui/src/ipc/mock.rs`, write unit tests asserting mock responses for `getStatus`, `listNodes`, `connect`, `disconnect`.

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p ghostpath-gui --target x86_64-unknown-linux-gnu`
Expected: FAIL (module not found).

- [ ] **Step 3: Implement Tauri 2 configuration, capabilities, and Mock RPC handler**
- Configure `crates/ghostpath-gui/Cargo.toml` with `tauri = "2"`, `tauri-build = "2"`.
- Create `tauri.conf.json` configuring window, offline bundle paths, and strict security settings.
- Implement `crates/ghostpath-gui/src/ipc/mock.rs` and `crates/ghostpath-gui/src/ipc/commands.rs`.
- Download/bundle local offline fonts (DM Sans, JetBrains Mono, Vazirmatn for Persian) in `UI/src/index.css` and remove external `fonts.googleapis.com` from `UI/index.html`.

- [ ] **Step 4: Run test to verify mock RPC passes**
Run: `cargo test -p ghostpath-gui --target x86_64-unknown-linux-gnu`
Expected: PASS.

- [ ] **Step 5: Verify UI build offline**
Run: `cd UI && npm run build`
Verify `dist/index.html` has no external `fonts.googleapis.com` or remote URL references.

- [ ] **Step 6: Commit Phase 2 deliverable**
```bash
git add crates/ghostpath-gui UI/index.html UI/src/index.css
git commit -m "feat(gui): implement Tauri 2 shell with mock RPC and offline bundled fonts"
```

---

### Task 3: Phase 3 - Real Unix Socket IPC Adapter & Event Streaming

**Files:**
- Create: `crates/ghostpath-gui/src/ipc/client.rs`
- Create: `crates/ghostpath-gui/src/ipc/events.rs`
- Modify: `crates/ghostpath-gui/src/ipc/commands.rs`
- Modify: `crates/ghostpath-gui/src/lib.rs`
- Modify: `UI/src/services/api.ts`
- Modify: `UI/src/App.tsx`

**Interfaces:**
- Consumes: `/run/ghostpath/ghostpathd.sock` JSON-RPC 2.0 stream
- Produces:
  - Rust: `rpc_request(method: String, params: Value) -> Result<Value, String>`
  - Tauri Events: `state_changed`, `traffic_updated`, `log_appended`
  - TypeScript: `GhostApi` with Tauri `invoke` and `listen` hooks

- [ ] **Step 1: Write failing unit test for Unix socket IPC client**
In `crates/ghostpath-gui/src/ipc/client.rs`, write unit tests connecting to a mock UnixListener simulating `getStatus` and streaming `traffic_updated` events.

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p ghostpath-gui --target x86_64-unknown-linux-gnu`
Expected: FAIL.

- [ ] **Step 3: Implement IPC client and event forwarder**
- Implement `IpcClient` in `crates/ghostpath-gui/src/ipc/client.rs` using `tokio::net::UnixStream`.
- Implement background task in `crates/ghostpath-gui/src/ipc/events.rs` that reads notifications and calls `app_handle.emit()`.
- Update `UI/src/services/api.ts` to replace HTTP `fetch` with `window.__TAURI__.core.invoke('rpc_request', { method, params })` and typed listeners.
- Update `UI/src/App.tsx` to subscribe to `traffic_updated` and `state_changed` instead of synthetic `setInterval`.

- [ ] **Step 4: Run test to verify IPC client passes**
Run: `cargo test -p ghostpath-gui --target x86_64-unknown-linux-gnu`
Expected: PASS.

- [ ] **Step 5: Commit Phase 3 deliverable**
```bash
git add crates/ghostpath-gui UI/src/services/api.ts UI/src/App.tsx
git commit -m "feat(gui): wire real Unix socket IPC adapter and streaming event bridge"
```

---

### Task 4: Phase 4 - Daemon State Handling & First-Run Diagnostics Screen

**Files:**
- Create: `crates/ghostpath-gui/src/ipc/daemon_status.rs`
- Modify: `crates/ghostpath-gui/src/ipc/commands.rs`
- Create: `UI/src/components/DaemonStatusBanner.tsx`
- Modify: `UI/src/App.tsx`
- Modify: `UI/src/components/DiagnosticsView.tsx`

**Interfaces:**
- Consumes: Socket check, filesystem permissions, version check
- Produces:
  - `DaemonState`: `Running | NotRunning | NotInstalled | PermissionDenied | VersionMismatch`
  - UI banner and action buttons ("Start Service via systemctl", "Fix Permissions", "Check Doctor")

- [ ] **Step 1: Write failing unit tests for daemon state detection**
Test classification of:
- Socket missing and `/usr/sbin/ghostpathd` absent -> `NotInstalled`
- Socket missing but binary exists -> `NotRunning`
- Socket exists with `EACCES` -> `PermissionDenied`
- Response version differs -> `VersionMismatch`

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p ghostpath-gui --target x86_64-unknown-linux-gnu`
Expected: FAIL.

- [ ] **Step 3: Implement daemon state checker and UI presentation**
- Implement `check_daemon_status()` command returning `{ state, message, fixCommand }`.
- Create `UI/src/components/DaemonStatusBanner.tsx` with non-blocking dismissible or persistent alert.
- Wire `DiagnosticsView.tsx` to display real `runDoctor` check results.

- [ ] **Step 4: Run test to verify daemon status passes**
Run: `cargo test -p ghostpath-gui --target x86_64-unknown-linux-gnu`
Expected: PASS.

- [ ] **Step 5: Commit Phase 4 deliverable**
```bash
git add crates/ghostpath-gui UI/src/components/DaemonStatusBanner.tsx UI/src/components/DiagnosticsView.tsx UI/src/App.tsx
git commit -m "feat(gui): add comprehensive daemon state detection and first-run diagnostics banner"
```

---

### Task 5: Phase 5 - System Tray, Single Instance, Notifications & Link Handler

**Files:**
- Modify: `crates/ghostpath-gui/Cargo.toml`
- Create: `crates/ghostpath-gui/src/tray.rs`
- Create: `crates/ghostpath-gui/src/single_instance.rs`
- Create: `crates/ghostpath-gui/src/deeplink.rs`
- Modify: `crates/ghostpath-gui/src/main.rs`
- Modify: `UI/src/components/AddProfileModal.tsx`
- Modify: `UI/src/App.tsx`

**Interfaces:**
- Consumes: System tray events, deep link URLs (`vless://`, `vmess://`, etc.), app lifecycle
- Produces:
  - Tray icon with status and quick connect/disconnect menu
  - Single instance focus on duplicate launch
  - System notifications on connect/disconnect/error
  - Deep link import dialog in UI

- [ ] **Step 1: Write failing unit tests for deep link URI parsing and tray menu builder**
Write tests asserting valid protocol URI interception and extraction.

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p ghostpath-gui --target x86_64-unknown-linux-gnu`
Expected: FAIL.

- [ ] **Step 3: Implement tray, single instance, notifications, and deep link handler**
- Configure `tauri-plugin-single-instance`, `tauri-plugin-notification`, `tauri-plugin-deep-link`.
- Build native system tray with items: Status, Quick Connect/Disconnect, Show Window, Quit.
- Configure window close handler to hide window instead of terminating process (with tray fallback check).
- Implement deep link listener forwarding imported links to `AddProfileModal.tsx` for user confirmation.

- [ ] **Step 4: Run tests to verify tray and deep link logic pass**
Run: `cargo test -p ghostpath-gui --target x86_64-unknown-linux-gnu`
Expected: PASS.

- [ ] **Step 5: Commit Phase 5 deliverable**
```bash
git add crates/ghostpath-gui UI/src/components/AddProfileModal.tsx UI/src/App.tsx
git commit -m "feat(gui): implement system tray, single-instance activation, notifications, and URI link handlers"
```

---

### Task 6: Phase 6 - Security Hardening, Capabilities & Strict CSP

**Files:**
- Modify: `crates/ghostpath-gui/tauri.conf.json`
- Modify: `crates/ghostpath-gui/capabilities/default.json`
- Modify: `UI/index.html`
- Create: `crates/ghostpath-gui/tests/security_tests.rs`

**Interfaces:**
- Consumes: Tauri security subsystem
- Produces: Hardened security configuration preventing arbitrary shell execution, remote network fetches, and devtools exposure in release

- [ ] **Step 1: Write failing test verifying CSP and zero remote network origins**
Write an automated test checking `tauri.conf.json` and built `UI/dist/index.html` asserting:
- `connect-src 'none'`
- No `http://` or `https://` script/style/font tags
- Shell/opener capabilities disabled for untrusted origins

- [ ] **Step 2: Run test to verify failure**
Run: `cargo test -p ghostpath-gui --test security_tests --target x86_64-unknown-linux-gnu`
Expected: FAIL.

- [ ] **Step 3: Implement capability lockdown and CSP verification**
- Enforce strict `default.json` capability permitting only custom IPC commands.
- Disable devtools when `debug_assertions` is false.
- Ensure all static assets, SVGs, and fonts reside locally within the bundle.

- [ ] **Step 4: Run security verification test**
Run: `cargo test -p ghostpath-gui --test security_tests --target x86_64-unknown-linux-gnu`
Expected: PASS.

- [ ] **Step 5: Commit Phase 6 deliverable**
```bash
git add crates/ghostpath-gui/tauri.conf.json crates/ghostpath-gui/capabilities/ crates/ghostpath-gui/tests/
git commit -m "security(gui): enforce strict CSP, locked-down capabilities, and offline asset guarantees"
```

---

### Task 7: Phase 7 - Linux Packaging (.deb, .rpm, AppImage, PKGBUILD)

**Files:**
- Modify: `packaging/debian/control`
- Modify: `packaging/debian/postinst`
- Modify: `packaging/debian/prerm`
- Modify: `packaging/debian/rules`
- Modify: `packaging/rpm/ghostpath-vpn.spec`
- Modify: `packaging/arch/PKGBUILD`
- Modify: `packaging/appimage/AppRun`
- Modify: `packaging/appimage/build-appimage.sh`
- Create: `packaging/ghostpath.metainfo.xml` (AppStream metadata)
- Create: `packaging/icons/` (16x16 up to 512x512 PNG + SVG)
- Create: `scripts/build_all_packages.sh`

**Interfaces:**
- Consumes: Built binaries (`ghostpath-gui`, `ghostpath`, `ghostpathd`), assets, systemd service
- Produces: Output packages in `dist/` (.deb, .rpm, AppImage, PKGBUILD archive)

- [ ] **Step 1: Write verification script for package metadata and files**
Create test validating package specs:
- Binary paths (`/usr/bin/ghostpath`, `/usr/bin/ghostpath-gui`, `/usr/sbin/ghostpathd`)
- Group creation (`ghostpath`) and user permissions
- Systemd service installation (`ghostpathd.service`)
- AppStream metadata validation and desktop MIME registration

- [ ] **Step 2: Run verification test to observe gaps**
Run: `cargo test -p ghostpath-core --target x86_64-unknown-linux-gnu`
Expected: Existing passes; package verification script needs implementation.

- [ ] **Step 3: Implement updated packaging definitions and build script**
- Update Debian `control`, `postinst`, `prerm` to install GUI, daemon, icons, and configure `ghostpath` group.
- Update RPM `.spec` and Arch `PKGBUILD` to package GUI and register desktop MIME handlers.
- Update `AppRun` and `build-appimage.sh` with explanatory first-run dialog about daemon installation.
- Provide SVG and PNG icon sizes (16, 24, 32, 48, 64, 128, 256, 512).
- Write `scripts/build_all_packages.sh`.

- [ ] **Step 4: Verify package generation and metadata consistency**
Run package build checks and verify `dist/` packaging layout.

- [ ] **Step 5: Commit Phase 7 deliverable**
```bash
git add packaging/ scripts/build_all_packages.sh
git commit -m "feat(packaging): provide full Linux packaging scripts for deb, rpm, AppImage, and Arch"
```

---

### Task 8: Phase 8 - End-to-End Tests, CI & Documentation

**Files:**
- Create: `tests/gui_e2e_tests.rs`
- Create: `.github/workflows/desktop_ci.yml`
- Create: `docs/DESKTOP_APP.md`
- Modify: `docs/API.md`
- Modify: `README.md`
- Modify: `README_FA.md`

**Interfaces:**
- Consumes: All completed phases
- Produces: Complete automated CI workflow, end-to-end integration tests, and comprehensive user/developer documentation

- [ ] **Step 1: Write integration tests for end-to-end GUI command lifecycle**
In `tests/gui_e2e_tests.rs`, test:
- Mock daemon launch -> GUI IPC handshake
- Connect command execution and status updates
- Disconnect command execution and cleanup
- Deep link URI import confirmation

- [ ] **Step 2: Run integration tests to verify failure**
Run: `cargo test --test gui_e2e_tests --target x86_64-unknown-linux-gnu`
Expected: FAIL.

- [ ] **Step 3: Implement end-to-end test runner, CI workflow, and documentation**
- Create `.github/workflows/desktop_ci.yml` matrix testing Ubuntu 22.04/24.04, Fedora, and Arch Linux.
- Write `docs/DESKTOP_APP.md` detailing architecture, IPC flows, capabilities, packaging, and Wayland/X11 troubleshooting.
- Update `README.md` and `README_FA.md` with desktop application instructions and screenshots.

- [ ] **Step 4: Run integration suite to verify all pass**
Run: `cargo test --workspace --target x86_64-unknown-linux-gnu`
Run: `cargo clippy --workspace --target x86_64-unknown-linux-gnu -- -D warnings`
Run: `cargo fmt --check`
Expected: All tests pass with 0 warnings.

- [ ] **Step 5: Commit Phase 8 deliverable**
```bash
git add tests/ .github/workflows/ docs/DESKTOP_APP.md README.md README_FA.md
git commit -m "test(gui): complete end-to-end integration tests, CI workflow, and desktop app docs"
```
