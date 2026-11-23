# AmneziaVPN & AmneziaWG Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable GhostPath VPN to import, validate, display, and connect with AmneziaVPN and AmneziaWG configurations (`vpn://` URIs, `.conf` files, `.vpn` files, and plain WireGuard configs) in Linux TUN mode with full policy routing, DNS protection, kill switch, and multi-VPN coexistence.

**Architecture:** 
1. New parser module `ghostpath-proto::amnezia` decodes `vpn://` (Base64URL + Qt `qCompress` 4-byte header + zlib) and AmneziaWG INI `.conf` files into strongly typed models with strict bounds, secret protection (`zeroize`), and explicit errors for unsupported containers.
2. New `TunnelBackend` abstraction alongside `Outbound` in `ghostpath-daemon` and `ghostpath-proto` to manage L3 network interfaces.
3. `AwgBackend` supervises `amneziawg-go` with socket fwmark `0x4750` for routing loop prevention via `table main`, binding tunnel DNS to `systemd-resolved`, integrating undo journals, and monitoring real handshake health and rate metrics.
4. UI and CLI support importing via file picker, paste, drag-and-drop, and deep links, displaying protocol badges and obfuscation summaries with zero fake data.

**Tech Stack:** Rust (Tokio, `thiserror`, `zeroize`, `flate2`/`miniz_oxide`, `base64`, `proptest`), `amneziawg-go` (userspace UAPI), React 19 + TypeScript (Tauri 2).

**Spec:** `docs/AMNEZIA.md` and `docs/DECISIONS.md#9`

## Global Constraints

- Never test network-mutating code on the host's real network namespace; use network namespaces (`unshare -n` / `ip netns`) and keep host tests behind `GHOSTPATH_ALLOW_HOST_TESTS=1`.
- Every mutation must have a guaranteed undo via RAII guards and `/var/lib/ghostpath/undo_journal.json`.
- Private keys and preshared keys must be stored with permissions `0600`, zeroized in memory (`zeroize`), masked in UI by default, and never logged.
- Decompression bounded to 2 MB maximum to prevent zip bombs.
- Zero mock code in production release builds.
- English and Persian (RTL) i18n support.

## Review Focus

1. **Hostile/Zip-Bomb decompression:** Malicious `vpn://` key specifying huge uncompressed length or infinite zlib stream must be rejected before allocating memory.
2. **Obfuscation parameter mismatch:** Invalid or conflicting parameters (`Jmin > Jmax`, duplicate headers `H1 == H2`, zero headers) must fail immediately with actionable error messages.
3. **Multi-container transparency:** Keys containing unsupported containers (e.g. `amnezia-openvpn-cloak`) must cleanly import supported containers while explicitly reporting skipped containers to the user.
4. **Clean SIGKILL recovery:** Abrupt termination of the daemon or `amneziawg-go` must leave no orphaned `awg-ghost0` interface, route rules, or DNS leaks.
5. **Multi-VPN coexistence:** Running an external WireGuard or OpenVPN instance inside or outside GhostPath must not suffer routing loops or broken connections.

---

### Task 1: Research, Verification & Architecture Documentation (Phase 1)

**Files:**
- Create: `docs/AMNEZIA.md`
- Modify: `docs/DECISIONS.md`

**Interfaces:**
- Produces: Complete verified protocol specification, parameter limits, container mapping table, and runtime comparison matrix in `docs/AMNEZIA.md`.

- [x] **Step 1: Verify upstream parameters and specifications**
  Verified against `amnezia-vpn/amnezia-client`, `amneziawg-go`, and `amnezia.org`.
- [x] **Step 2: Document architectural decisions**
  Recorded verified findings in `docs/AMNEZIA.md` and Decision 9 in `docs/DECISIONS.md`.
- [x] **Step 3: Commit**
  ```bash
  git add docs/AMNEZIA.md docs/DECISIONS.md
  git commit -m "docs: document AmneziaVPN and AmneziaWG technical specifications and decisions"
  ```

---

### Task 2: AmneziaWG Data Models & Obfuscation Types in `ghostpath-proto` (Phase 2.1)

**Files:**
- Create: `crates/ghostpath-proto/src/amnezia/model.rs`
- Modify: `crates/ghostpath-proto/src/amnezia/mod.rs`
- Modify: `crates/ghostpath-proto/src/lib.rs`
- Test: `crates/ghostpath-proto/tests/amnezia_model_test.rs`

**Interfaces:**
- Produces: `AwgObfuscationConfig`, `AwgInterfaceConfig`, `AwgPeerConfig`, `AwgProfileConfig`, `AmneziaContainerConfig`, `AmneziaServerConfig`.

- [ ] **Step 1: Write unit tests for AWG model validation and constraints**
  Test constraints: `Jmin <= Jmax`, `H1..H4` non-zero and pairwise distinct, key lengths (32 bytes), MTU ranges (1280–1500).
- [ ] **Step 2: Run test to verify it fails**
  `cargo test -p ghostpath-proto --test amnezia_model_test` -> FAIL
- [ ] **Step 3: Implement data structures in `model.rs`**
  Add typed structs with serde serialization, validation logic, and `zeroize::ZeroizeOnDrop` for private keys.
- [ ] **Step 4: Run test to verify it passes**
  `cargo test -p ghostpath-proto --test amnezia_model_test` -> PASS
- [ ] **Step 5: Commit**
  ```bash
  git add crates/ghostpath-proto/src/amnezia/ crates/ghostpath-proto/tests/
  git commit -m "feat(proto): add AmneziaWG configuration data models and obfuscation validators"
  ```

---

### Task 3: INI `.conf` Parser and Serializer for AmneziaWG and WireGuard (Phase 2.2)

**Files:**
- Create: `crates/ghostpath-proto/src/amnezia/ini_parser.rs`
- Test: `crates/ghostpath-proto/tests/amnezia_ini_test.rs`

**Interfaces:**
- Consumes: `AwgProfileConfig` from Task 2.
- Produces: `parse_awg_conf(input: &str) -> Result<AwgProfileConfig, AmneziaParseError>` and `export_awg_conf(config: &AwgProfileConfig) -> String`.

- [ ] **Step 1: Write failing golden tests with synthetic AWG and WG configs**
  Test parsing full AmneziaWG v1/v2 `.conf` and standard WireGuard `.conf` without obfuscation.
- [ ] **Step 2: Run test to verify it fails**
  `cargo test -p ghostpath-proto --test amnezia_ini_test` -> FAIL
- [ ] **Step 3: Implement INI parser and exporter in `ini_parser.rs`**
  Robust line-by-line section parser for `[Interface]` and `[Peer]` handling addresses, DNS, MTU, `Jc`, `Jmin`, `Jmax`, `S1`..`S4`, `H1`..`H4`, and `I1`..`I5`.
- [ ] **Step 4: Run test to verify it passes**
  `cargo test -p ghostpath-proto --test amnezia_ini_test` -> PASS
- [ ] **Step 5: Commit**
  ```bash
  git add crates/ghostpath-proto/src/amnezia/ini_parser.rs crates/ghostpath-proto/tests/amnezia_ini_test.rs
  git commit -m "feat(proto): implement AmneziaWG and WireGuard INI config parser and exporter"
  ```

---

### Task 4: `vpn://` URI & Qt `qCompress` Decompressor with Zip-Bomb Bounds (Phase 2.3)

**Files:**
- Create: `crates/ghostpath-proto/src/amnezia/vpn_uri.rs`
- Test: `crates/ghostpath-proto/tests/amnezia_uri_test.rs`

**Interfaces:**
- Produces: `parse_vpn_uri(uri: &str) -> Result<AmneziaServerConfig, AmneziaParseError>` and `encode_vpn_uri(server: &AmneziaServerConfig) -> Result<String, AmneziaParseError>`.

- [ ] **Step 1: Write failing unit and hostile-input tests for `vpn://` decoding**
  Test valid Base64URL + 4-byte BE length + zlib, truncated bytes, invalid zlib, size header > 2MB (must reject before decompressing).
- [ ] **Step 2: Run test to verify it fails**
  `cargo test -p ghostpath-proto --test amnezia_uri_test` -> FAIL
- [ ] **Step 3: Implement `vpn_uri.rs` using `base64` and `flate2`**
  Enforce 2 MB decompression limit using bounded reader; parse JSON server structure, extracting containers (`awg`, `xray`, `shadowsocks`, etc.) and flagging unsupported ones.
- [ ] **Step 4: Run test to verify it passes**
  `cargo test -p ghostpath-proto --test amnezia_uri_test` -> PASS
- [ ] **Step 5: Commit**
  ```bash
  git add crates/ghostpath-proto/src/amnezia/vpn_uri.rs crates/ghostpath-proto/tests/amnezia_uri_test.rs
  git commit -m "feat(proto): implement safe bounded vpn:// Qt qCompress decoder and encoder"
  ```

---

### Task 5: Property and Fuzz Testing for Amnezia Parsers (Phase 2.4)

**Files:**
- Create: `crates/ghostpath-proto/tests/amnezia_proptest.rs`

**Interfaces:**
- Consumes: Parsers from Tasks 2, 3, and 4.
- Produces: Continuous fuzz and roundtrip property tests.

- [ ] **Step 1: Write proptests for arbitrary input and round-trip fidelity**
  Test that `parse_awg_conf(export_awg_conf(cfg)) == cfg` and random byte inputs never cause panics or unbounded memory growth.
- [ ] **Step 2: Run proptests**
  `cargo test -p ghostpath-proto --test amnezia_proptest` -> PASS
- [ ] **Step 3: Commit**
  ```bash
  git add crates/ghostpath-proto/tests/amnezia_proptest.rs
  git commit -m "test(proto): add property-based fuzz tests for Amnezia parsers"
  ```

---

### Task 6: `TunnelBackend` Trait & `AwgBackend` Userspace Architecture (Phase 3)

**Files:**
- Create: `crates/ghostpath-daemon/src/tunnel/mod.rs`
- Create: `crates/ghostpath-daemon/src/tunnel/backend.rs`
- Create: `crates/ghostpath-daemon/src/tunnel/awg.rs`
- Modify: `crates/ghostpath-daemon/src/lib.rs`
- Test: `crates/ghostpath-daemon/tests/awg_backend_test.rs`

**Interfaces:**
- Produces: `pub trait TunnelBackend: Send + Sync { async fn start(&mut self, config: &AwgProfileConfig, journal: &mut UndoJournal) -> Result<()>; async fn stop(&mut self, journal: &mut UndoJournal) -> Result<()>; async fn status(&self) -> TunnelStats; }`.
- Implements: `AwgBackend` supervising `amneziawg-go`, writing UAPI parameters over Unix domain socket.

- [ ] **Step 1: Write failing test in isolated network namespace for `AwgBackend` lifecycle**
  Test interface creation, UAPI configuration, address assignment, and clean teardown.
- [ ] **Step 2: Run test to verify it fails**
  `cargo test -p ghostpath-daemon --test awg_backend_test` -> FAIL
- [ ] **Step 3: Implement `TunnelBackend` and `AwgBackend` in `ghostpath-daemon`**
  Manage `amneziawg-go` process with `WG_PROCESS_MARK=0x4750`, configure UAPI (`private_key`, `public_key`, `endpoint`, `allowed_ip`, `jc`, `jmin`, `jmax`, `s1`..`s4`, `h1`..`h4`), assign interface address and MTU.
- [ ] **Step 4: Run test to verify it passes in network namespace**
  `cargo test -p ghostpath-daemon --test awg_backend_test` -> PASS
- [ ] **Step 5: Commit**
  ```bash
  git add crates/ghostpath-daemon/src/tunnel/ crates/ghostpath-daemon/tests/awg_backend_test.rs
  git commit -m "feat(daemon): implement TunnelBackend abstraction and AwgBackend userspace engine"
  ```

---

### Task 7: Routing, DNS, Kill Switch & Namespace Integration Tests (Phase 4 & 5)

**Files:**
- Modify: `crates/ghostpath-daemon/src/engine.rs`
- Modify: `crates/ghostpath-net/src/manager.rs`
- Test: `crates/ghostpath-daemon/tests/awg_namespace_integration_test.rs`

**Interfaces:**
- Integrates `AwgBackend` with `RouteManager`, `DnsManager`, `KillSwitch`, and `UndoJournal`.

- [ ] **Step 1: Write failing namespace integration test for AWG traffic and leak prevention**
  Assert handshake, bidirectional ping through AWG tunnel, zero DNS leaks, zero IPv6 leaks, and correct route rule priorities (`5180`/`5181`).
- [ ] **Step 2: Run test to verify it fails**
  `cargo test -p ghostpath-daemon --test awg_namespace_integration_test` -> FAIL
- [ ] **Step 3: Wire `AwgBackend` into `DaemonEngine`**
  Configure policy routing table `5180` with default route `dev awg-ghost0`, exclude UDP endpoint via fwmark `0x4750`, bind DNS to `systemd-resolved`, record undo journal entries.
- [ ] **Step 4: Run test to verify it passes**
  `cargo test -p ghostpath-daemon --test awg_namespace_integration_test` -> PASS
- [ ] **Step 5: Commit**
  ```bash
  git add crates/ghostpath-daemon/src/engine.rs crates/ghostpath-net/ crates/ghostpath-daemon/tests/
  git commit -m "feat(daemon): integrate AwgBackend with policy routing, DNS, and undo journal"
  ```

---

### Task 8: Daemon IPC Protocol, CLI Commands & `doctor` Diagnostics (Phase 6)

**Files:**
- Modify: `crates/ghostpath-daemon/src/ipc/protocol.rs`
- Modify: `crates/ghostpath-daemon/src/doctor.rs`
- Modify: `crates/ghostpath-cli/src/main.rs`
- Modify: `docs/API.md`
- Test: `crates/ghostpath-daemon/tests/awg_ipc_test.rs`

**Interfaces:**
- Produces: `ghostpath import <vpn://... | file.conf | file.vpn>`, profile listing with AWG badges, and `ghostpath doctor` checking `amneziawg-go` binary, `/dev/net/tun`, and kernel module status.

- [ ] **Step 1: Write failing test for AWG IPC commands and doctor checks**
  Test `import_profile` with Amnezia inputs and `doctor` output for AWG requirements.
- [ ] **Step 2: Run test to verify it fails**
  `cargo test -p ghostpath-daemon --test awg_ipc_test` -> FAIL
- [ ] **Step 3: Implement IPC methods, CLI handlers, and doctor diagnostics**
  Handle `vpn://` and `.conf` in `import` command, format AWG stats in `status`, add AWG binary checks in `doctor`.
- [ ] **Step 4: Run test to verify it passes**
  `cargo test -p ghostpath-daemon --test awg_ipc_test` -> PASS
- [ ] **Step 5: Commit**
  ```bash
  git add crates/ghostpath-daemon/src/ipc/ crates/ghostpath-daemon/src/doctor.rs crates/ghostpath-cli/ docs/API.md
  git commit -m "feat(cli): add Amnezia import, status reporting, and doctor checks"
  ```

---

### Task 9: Desktop GUI Import Flow, Protocol Badges & Unsupported Container Feedback (Phase 7)

**Files:**
- Modify: `UI/src/components/AddProfileModal.tsx`
- Modify: `UI/src/components/TunnelView.tsx`
- Modify: `UI/src/types/index.ts`
- Modify: `crates/ghostpath-gui/src/deeplink.rs`
- Modify: `crates/ghostpath-gui/src/ipc/commands.rs`
- Test: `crates/ghostpath-gui/tests/deeplink_tests.rs`

**Interfaces:**
- Consumes: IPC import methods and status events from Task 8.
- Produces: Import dialog supporting `vpn://`, `.conf`, `.vpn` files, displaying `AmneziaWG` protocol badges, obfuscation parameters in read-only section, and dialog reporting skipped unsupported containers.

- [ ] **Step 1: Write failing deep link and GUI command test for `vpn://` handling**
  Test deep link parsing for `vpn://` links and command forwarding.
- [ ] **Step 2: Run test to verify it fails**
  `cargo test -p ghostpath-gui --test deeplink_tests` -> FAIL
- [ ] **Step 3: Implement UI and GUI deep link handlers**
  Update `AddProfileModal.tsx` to accept `vpn://` keys and files, update `TunnelView.tsx` with AWG badge and obfuscation details, register `vpn://` MIME scheme handler in Tauri.
- [ ] **Step 4: Run test and build UI**
  `cargo test -p ghostpath-gui --test deeplink_tests` -> PASS
  `npm --prefix UI run build` -> PASS
- [ ] **Step 5: Commit**
  ```bash
  git add UI/ crates/ghostpath-gui/
  git commit -m "feat(ui): add Amnezia profile import modal, protocol badges, and deep link handler"
  ```

---

### Task 10: Packaging with Bundled `amneziawg-go`, Multi-Distro CI & Docs (Phase 8)

**Files:**
- Modify: `packaging/build-deb.sh`
- Modify: `packaging/appimage/build-appimage.sh`
- Modify: `packaging/arch/PKGBUILD`
- Modify: `packaging/rpm/ghostpath-vpn.spec`
- Modify: `README.md`
- Modify: `README_FA.md`
- Modify: `scripts/build_all_packages.sh`

**Interfaces:**
- Produces: Rebuilt `.deb`, `.rpm`, `AppImage`, and `PKGBUILD` bundling `amneziawg-go` with license notices, passing zero-demo scanner, and updated documentation in English and Persian.

- [ ] **Step 1: Update packaging scripts to bundle `amneziawg-go` and license notices**
  Install `amneziawg-go` to `/usr/bin/amneziawg-go` in `.deb`, AppImage, RPM, and Arch PKGBUILD.
- [ ] **Step 2: Run verification scanner and full workspace tests**
  `./scripts/verify_no_demo_content.sh` -> PASS
  `cargo test --workspace` -> PASS
  `cargo clippy --workspace -- -D warnings` -> PASS
- [ ] **Step 3: Rebuild distribution packages and export `.deb` to root**
  `bash scripts/build_all_packages.sh`
  `cp dist/ghostpath-vpn_0.1.0_amd64.deb ./ghostpath_0.1.0_amd64.deb`
- [ ] **Step 4: Commit**
  ```bash
  git add packaging/ README.md README_FA.md scripts/
  git commit -m "chore(packaging): bundle amneziawg-go, update multi-distro packages and documentation"
  ```
