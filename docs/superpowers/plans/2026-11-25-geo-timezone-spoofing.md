# Geo Location & System Timezone Spoofing Implementation Plan

> **For agentic workers:** Follow `.agents/rules/superpowers.md` phase by phase. Commit after each verified phase. Never test destructive OS timezone/geoclue mutations on the host directly without dry-run/temp-root isolation.

**Goal:** Enable users to select any country (or auto-match the active VPN server's country) in the GhostPath Routing view or CLI to safely spoof Linux system timezone (`/etc/localtime` / `timedatectl`) and desktop geolocation (`GeoClue2` `/etc/geolocation` & drop-in config) with full `UndoJournal` auto-restore on disconnect or crash recovery, alongside fixed dark-theme Split Tunneling & Rules controls.

---

### Phase 1: Core Country Catalog, Settings & Journal Types (`ghostpath-core` & `ghostpath-net`)
**Files:**
- Create: `crates/ghostpath-core/src/geo.rs`
- Modify: `crates/ghostpath-core/src/lib.rs`, `crates/ghostpath-core/src/models.rs`, `crates/ghostpath-core/src/paths.rs`
- Create: `crates/ghostpath-net/src/geo_spoof.rs`
- Modify: `crates/ghostpath-net/src/journal.rs`, `crates/ghostpath-net/src/lib.rs`

**Steps:**
1. Add `GeoCountryPreset`, `GeoSpoofMode` (`Manual`, `MatchNode`), `GeoSpoofSettings`, `GeoSpoofState` in `ghostpath-core::geo` and wire `geo_spoof: GeoSpoofSettings` into `AppSettings` (defaulting `enabled: false`).
2. Add helper paths in `GhostPaths` (`timezone_backup()`, `geolocation_backup()`).
3. Add `JournalAction::ModifiedTimezone` and `JournalAction::ModifiedGeolocation` to `UndoJournal`.
4. Implement `GeoSpoofManager` + `GeoSpoofGuard` in `ghostpath-net::geo_spoof` supporting:
   - Custom root / dry-run path injection (`GeoSpoofPaths`) so unit & integration tests exercise real symlink swaps, `/etc/timezone` writes, `/etc/geolocation` writes, `/etc/geoclue/conf.d/99-ghostpath.conf` writes, backup preservation, and full rollback on `Drop` or `restore()` without touching host `/etc`.
   - Real Linux backend (`timedatectl set-timezone` + `/etc/localtime` & `/etc/timezone` verification/fallback, GeoClue drop-in + `systemctl try-restart geoclue`).
   - Reading live system timezone and active geolocation state.
5. Verify with `cargo test -p ghostpath-core -p ghostpath-net` and commit.

---

### Phase 2: Daemon Engine, IPC, CLI, Doctor & Recovery (`ghostpath-daemon`, `ghostpath-cli`, `systemd`)
**Files:**
- Modify: `crates/ghostpath-daemon/src/engine.rs`
- Modify: `crates/ghostpath-daemon/src/ipc/handler.rs`
- Modify: `crates/ghostpath-daemon/src/main.rs`
- Modify: `crates/ghostpath-daemon/src/doctor.rs`
- Modify: `crates/ghostpath-cli/src/main.rs`
- Modify: `systemd/ghostpathd.service`

**Steps:**
1. Store `geo_spoof_guard: Option<GeoSpoofGuard>` in `ActiveSession` and manage manual vs session spoofing cleanly in `DaemonEngine`.
2. Auto-apply geo/timezone spoofing on `connect()` when `config.settings.geo_spoof.enabled && apply_on_connect` (resolving `MatchNode` country via `node.country_code` / `node.country`), and auto-restore in `disconnect()`.
3. Implement IPC methods: `listGeoCountries`, `getGeoSpoofStatus`, `updateGeoSpoof`, `applyGeoSpoof`, `restoreGeoSpoof`, and `updateRuleAction` (notice `handle_change_rule_action` in UI currently wasn't persisted to daemon — fix that too!).
4. Add `ghostpathd --recover` rollback support for `ModifiedTimezone` and `ModifiedGeolocation`.
5. Add `check_geo_timezone_support()` in `SystemDoctor` and `ghostpath geo <status|set|restore>` CLI subcommands.
6. Update `systemd/ghostpathd.service` `ReadWritePaths` for `/etc/localtime`, `/etc/timezone`, `/etc/geolocation`, `/etc/geoclue/conf.d`.
7. Verify with unit/IPC tests (`cargo test -p ghostpath-daemon -p ghostpath-cli`) and commit.

---

### Phase 3: Desktop UI Geo Location & Timezone Spoofing + Dark Dropdown Fix (`UI/` & `ghostpath-gui`)
**Files:**
- Modify: `UI/src/index.css`
- Modify: `UI/src/types/vpn.ts`
- Modify: `UI/src/data/vpnData.ts`
- Modify: `UI/src/services/api.ts`
- Modify: `UI/src/components/RoutingView.tsx`
- Modify: `UI/src/components/SettingsView.tsx`
- Modify: `UI/src/App.tsx`
- Modify: `crates/ghostpath-gui/src/ipc/mock.rs` (test-only mock parity)

**Steps:**
1. Set `color-scheme: dark;` in `UI/src/index.css` and fix `<select>` + `<option>` contrast classes across `RoutingView.tsx` and `SettingsView.tsx` so dropdowns render in dark surface colors with crisp readable text.
2. Add the **Geo Location & System Timezone Spoofing** panel inside `RoutingView.tsx` with:
   - Master spoofing toggle
   - Mode switcher (`Match VPN Server Country` vs `Select Country Manually`)
   - Country selector dropdown (with country code, city, timezone, coordinates)
   - Individual toggles for System Timezone (`timedatectl`/`/etc/localtime`), Desktop Geo Location (`GeoClue2`), Auto-Restore on Disconnect, and Strict Geo Routing Alignment
   - Live OS readout (current system timezone, local clock preview, active GeoClue coordinates, backed-up original timezone)
   - Explicit `Apply Now` and `Restore Original` action buttons
3. Wire `GhostApi` methods and real-time refresh in `App.tsx` (also persist rule action changes via `updateRuleAction`).
4. Update `ghostpath-gui` test mock and add GUI tests for the new RPC methods; build UI (`npm run build`), run zero-demo check, full workspace `cargo test` & `clippy`, rebuild `.deb` and export to root, then commit.
