# GhostPath VPN Security & Correctness Findings

**Auditor Mode:** Hostile external audit  
**Baseline Commit:** `37cb98b`  
**Status Legend:** Findings are audited from source, configuration, packaging scripts, and live verification tools. Each Critical/High/Medium finding and cheap Low finding includes a concrete remediation and a failing-before / passing-after regression test.

---

## Summary Table

| ID | Severity | Area | Title | Location | Status |
|---|---|---|---|---|---|
| GP-SEC-01 | **Critical** | C / D | Silent fail-open to mock TUN, mock SOCKS5, and unverified `Connected` state | `crates/ghostpath-tun/src/lib.rs:67-71`, `crates/ghostpath-proto/src/outbound/managed.rs:125-131`, `crates/ghostpath-daemon/src/engine.rs:166-290` | Fixed / Pending Verification |
| GP-SEC-02 | **Critical** | C / D | Kill switch blocks `awg-ghost0`, runs after routing changes (startup leak window), and does not block mDNS/LLMNR or enforce pre-connect fail-closed | `crates/ghostpath-net/src/killswitch.rs:116-126`, `crates/ghostpath-daemon/src/engine.rs:185-261` | Fixed / Pending Verification |
| GP-SEC-03 | **Critical** | A / C | Predictable world-readable `/tmp/ghostpath_core_{port}.json` exposes private keys/UUIDs/Reality secrets | `crates/ghostpath-proto/src/outbound/managed.rs:31` | Fixed / Pending Verification |
| GP-SEC-04 | **Critical** | D / C | Non-functional `ghost0` TUN-to-core dataplane & stub `DnsProxy` echo-only responder | `crates/ghostpath-daemon/src/engine.rs:190-290`, `crates/ghostpath-net/src/dns_proxy.rs:1-110` | Fixed / Pending Verification |
| GP-SEC-05 | **High** | B / C | SSRF, missing TLS/redirect/size bounds, and plaintext URL logging in subscription fetcher | `crates/ghostpath-daemon/src/engine.rs:843-980` | Fixed / Pending Verification |
| GP-SEC-06 | **High** | A / D | Crash recovery (`ghostpathd --recover`) ignores routing/DNS/killswitch `UndoJournal` | `crates/ghostpath-daemon/src/main.rs:46-63` | Fixed / Pending Verification |
| GP-SEC-07 | **High** | B / A | Newline UAPI injection & CLI `-` option injection in AmneziaWG and Coexistence (`ip`/`nft`) | `crates/ghostpath-daemon/src/tunnel/awg.rs`, `crates/ghostpath-net/src/coexistence.rs`, `crates/ghostpath-net/src/manager.rs` | Fixed / Pending Verification |
| GP-SEC-08 | **High** | A | Over-permissive IPC socket mode (`0o666`), `adm` group trust, unbounded line DoS, unlimited concurrent clients | `crates/ghostpath-daemon/src/ipc/server.rs:42-142`, `crates/ghostpath-daemon/src/ipc/auth.rs:80` | Fixed / Pending Verification |
| GP-SEC-09 | **High** | A / C | Permissions race (`0o644` -> `0o600`), non-atomic writes, `/tmp` fallback, and symlink/TOCTOU risk on config/state/geo files | `crates/ghostpath-core/src/config.rs:64-78`, `crates/ghostpath-core/src/paths.rs:65`, `crates/ghostpath-net/src/geo_spoof.rs`, `crates/ghostpath-net/src/dns.rs` | Fixed / Pending Verification |
| GP-SEC-10 | **High** | C / D | Silent bypass of `.ir` domains/IPs when routing rules are empty & unencrypted SOCKS fallback on malformed URI | `crates/ghostpath-proto/src/config_gen/singbox.rs:35-39,234-241` | Fixed / Pending Verification |
| GP-SEC-11 | **Medium** | A | Unprivileged `systemd` unit exposure score `6.0 MEDIUM` and missing daemon `PR_SET_DUMPABLE`/core-dump/umask hardening | `systemd/ghostpathd.service:1-46`, `crates/ghostpath-daemon/src/main.rs` | Fixed / Pending Verification |
| GP-SEC-12 | **Medium** | A / C | Unvalidated IPC `updateSettings` parameters (`mtu` integer truncation, unvalidated `tunStack`/`dnsPrimary`/`excludedInterfaces`) and raw secret exposure in `ServerNode::raw_link` | `crates/ghostpath-daemon/src/engine.rs:715-765`, `crates/ghostpath-core/src/models.rs:72` | Fixed / Pending Verification |
| GP-SEC-13 | **Medium** | B | Parser bounds & validation gaps (`ini_parser` unbounded input, `subscription.rs` `u64 as u16` port wrap & `127.0.0.1` fallback, `vmess`/`ss`/`vless` validation) | `crates/ghostpath-proto/src/amnezia/ini_parser.rs`, `crates/ghostpath-proto/src/parser/{subscription,vmess,ss,vless}.rs` | Fixed / Pending Verification |
| GP-SEC-14 | **Medium** | F / B | Deep link lacks user confirmation summary dialog & spam rate-limit; Tauri CSP has `unsafe-inline` styles & `withGlobalTauri: true` without explicit nav guard | `UI/src/App.tsx:247-250`, `crates/ghostpath-gui/tauri.conf.json:10,25`, `crates/ghostpath-gui/src/main.rs` | Fixed / Pending Verification |
| GP-SEC-15 | **Medium** | C | Plaintext keys retained in `AwgBackend::last_config` without `ZeroizeOnDrop`; raw URI logged/leaked across errors | `crates/ghostpath-daemon/src/tunnel/awg.rs:22`, `crates/ghostpath-proto/src/amnezia/model.rs` | Fixed / Pending Verification |
| GP-SEC-16 | **Medium** | D | `ghostpath-tun` FD leak on `ioctl` error, ignored `ip` command failures, `CoexistenceManager` never invoked in `connect()` | `crates/ghostpath-tun/src/lib.rs:207-246`, `crates/ghostpath-daemon/src/engine.rs` | Fixed / Pending Verification |
| GP-SEC-17 | **Medium** | E / G | Bundled `packaging/bin/amneziawg-go` lacks build-time SHA-256 verification; RPM spec misses binary in `%files` & Arch `sha256sums=('SKIP')`; Debian `postinst`/`prerm`/`postrm` lifecycle gaps | `packaging/build-deb.sh`, `packaging/rpm/ghostpath-vpn.spec`, `packaging/arch/PKGBUILD`, `packaging/debian/{postinst,prerm}` | Fixed / Pending Verification |
| GP-SEC-18 | **Low** | E | Missing `#![forbid(unsafe_code)]` on safe crates, `clippy::pedantic` failures, `expect()` calls in `ghostpathd/main.rs`, missing `deny.toml`/SBOM/`cargo-audit` config | `crates/*/src/{lib,main}.rs`, `crates/ghostpath-daemon/src/main.rs:106-108` | Fixed / Pending Verification |
| GP-SEC-19 | **Low** | F | UI `control character` sanitization in notifications/logs, RTL (`dir="auto"`) for Persian remarks, `initial settings` mismatch with backend `1.1.1.1` vs DoH | `UI/src/App.tsx`, `UI/src/data/vpnData.ts:45-46` | Fixed / Pending Verification |
| GP-SEC-20 | **Info** | H | Missing `SECURITY.md`, `docs/THREAT_MODEL.md` / `docs/TROUBLESHOOTING.md`, and CI security workflow jobs | Repository root & `.github/workflows/desktop_ci.yml` | Fixed / Pending Verification |

---

## Detailed Findings

### GP-SEC-01 (Critical — CVSS 9.1): Silent Fail-Open to Mock TUN, Mock SOCKS5 Listener, and Unverified `Connected` State
- **Area:** C (Secrets & Privacy) / D (Networking Correctness)
- **Location:** `crates/ghostpath-tun/src/lib.rs:67-71`, `crates/ghostpath-proto/src/outbound/managed.rs:125-131`, `crates/ghostpath-daemon/src/engine.rs:166-290`
- **Description:**
  1. `TunDevice::open()` catches errors opening `/dev/net/tun` and sets `self.is_mock = true`, returning `Ok(())`.
  2. `ManagedCoreOutbound::start()` silently spawns `spawn_mock_socks_listener()` whenever `sing-box` or `xray` is missing from `PATH`, returning `Ok(())` even though it immediately closes accepted sockets cleanly after greeting and forwards zero traffic.
  3. `DaemonEngine::connect()` logs warnings (`if let Err(e) = ... { warn!(...) }`) when TUN creation, policy routing, DNS, kill switch, `AwgBackend`, or `ManagedCoreOutbound` fail, and unconditionally transitions `StateManager` to `ConnectionStatus::Connected`.
- **Exploit / Failure Scenario:** Under censored environments, a user missing `sing-box`, suffering an `nft` failure, or running without `CAP_NET_ADMIN` clicks **Connect**, receives a green **"Connected"** state, and assumes traffic is encrypted while traffic fails open or leaks directly via default OS interfaces/DNS.
- **Fix:** Gate mock TUN/mock SOCKS strictly behind `#[cfg(any(test, feature = "dev-mock"))]` (never compiled into default/release builds) OR `dry_run` mode; fail closed in `DaemonEngine::connect()` and immediately roll back partial setup (KillSwitch, routes, DNS, core child) transitioning to `ConnectionStatus::Failed` with sanitized diagnostics whenever any required stage fails.
- **Regression Test:** `tests::test_connect_fails_closed_when_core_or_tun_unavailable`, `tun::tests::test_open_refuses_mock_fallback_in_production`, `managed::tests::test_missing_binary_returns_error`.

### GP-SEC-02 (Critical — CVSS 8.8): Kill Switch Blocks `awg-ghost0`, Arms After Route/DNS Setup (Startup Leak Window), and Omits mDNS/LLMNR Blocks
- **Area:** C (Privacy / Leak Prevention) / D (Networking Correctness)
- **Location:** `crates/ghostpath-net/src/killswitch.rs:116-126`, `crates/ghostpath-daemon/src/engine.rs:185-261`
- **Description:**
  1. `KillSwitch::enable()` hardcodes `oifname "ghost0" accept`, completely blocking egress packets when AmneziaWG/WireGuard creates `awg-ghost0`.
  2. `DaemonEngine::connect()` sets routes, DNS, and starts the proxy core *before* arming `KillSwitch::enable()`, leaving a startup window where plaintext queries/connections leave via the physical interface.
  3. Unicast DNS ports UDP/TCP 53, DoT (853), mDNS (UDP 5353), and LLMNR (UDP/TCP 5355) on non-tunnel interfaces are not explicitly blocked in the nftables table (relying only on default output drop if enabled, while ICMP/LAN allow rules need strict scoping and explicit non-tunnel DNS drop rules).
- **Exploit / Failure Scenario:** User enables Kill Switch and connects to an AmneziaWG server (`awg-ghost0`): internet completely breaks or packets sent during connect handshake leak over local ISP prior to nftables rule load.
- **Fix:** Parameterize `KillSwitch::enable(iface_name, ...)` with validated tunnel interface name (`ghost0` or `awg-ghost0`), arm KillSwitch **before** configuring DNS/routes (with rollback on connect failure), explicitly drop non-loopback/non-tunnel port 53/853/5353/5355 traffic, and verify via `unshare -n` namespace nftables inspection.
- **Regression Test:** `killswitch::tests::test_killswitch_accepts_dynamic_tun_iface_and_blocks_leaks` (+ `unshare -n` integration test).

### GP-SEC-03 (Critical — CVSS 8.2): World-Readable `/tmp/ghostpath_core_{port}.json` Exposes Private Proxy Credentials
- **Area:** A (Privilege Boundary) / C (Secrets)
- **Location:** `crates/ghostpath-proto/src/outbound/managed.rs:31`
- **Description:** `ManagedCoreOutbound::new` places core runtime config at `std::env::temp_dir().join(format!("ghostpath_core_{socks_port}.json"))` and `tokio::fs::write` writes it with default `0o644` permissions inside sticky world-writable `/tmp`.
- **Exploit / Failure Scenario:** Any unprivileged local user (`T1`) on a multi-user system reads `/tmp/ghostpath_core_10808.json` to extract VLESS UUIDs, Reality public keys/short IDs, Trojan/Shadowsocks passwords, and server endpoints, or pre-populates/symlinks the predictable filename before `ghostpathd` writes it.
- **Fix:** Store runtime core configs in a root/daemon-owned `0o700` runtime directory (`/run/ghostpath/cores/` or `GhostPaths::runtime_core_dir()`), create unique temp files via `O_CREAT | O_EXCL | O_NOFOLLOW` with mode `0o600`, write and unlink/zeroize cleanly on `stop()` and `Drop`.
- **Regression Test:** `managed::tests::test_core_config_written_with_0600_and_non_tmp_dir`.

### GP-SEC-04 (Critical — CVSS 8.6): Disconnected `ghost0` TUN-to-Core Dataplane & Echo-Only `DnsProxy`
- **Area:** C (DNS Leak) / D (Networking Correctness)
- **Location:** `crates/ghostpath-daemon/src/engine.rs:190-290`, `crates/ghostpath-net/src/dns_proxy.rs:1-110`
- **Description:**
  1. For VLESS/VMess/Trojan/SS in `TUN` mode, `DaemonEngine::connect()` opens `/dev/net/tun` (`ghost0`) and starts `sing-box` listening ONLY on `127.0.0.1:10808` (SOCKS5) / `10809` (mixed), without any TUN-to-SOCKS packet pump (`tun2socks`) or `sing-box` `tun` inbound. Traffic routed into `ghost0` hits a dead queue unless forwarded.
  2. `DnsProxy` in `crates/ghostpath-net/src/dns_proxy.rs` merely flips DNS header bytes (`QR=1, RA=1`) and echoes empty responses with zero records back to clients instead of resolving via the encrypted tunnel or DoH!
- **Exploit / Failure Scenario:** Connecting in TUN mode silently blackholes user TCP/UDP flows or fails all DNS resolution with empty `NOERROR` 0-answer responses, causing fallback resolvers or user manual bypass.
- **Fix:** Configure `sing-box` TUN inbound (`ghost0`, `routing_mark` / `auto_route: false`, `auto_detect_interface: false`, strict MTU & `stack`) when running in TUN mode (managed by `ghostpathd`'s `RouteManager` + `KillSwitch` so policy routing & nftables ownership remain deterministic and race-free) OR pass pre-opened/cleanly managed interface, and upgrade `DnsProxy` to forward DNS queries over TCP/SOCKS5 (`127.0.0.1:socks_port`) or HTTPS DoH upstream through the tunnel instead of returning empty stub replies.
- **Regression Test:** `singbox::tests::test_tun_inbound_generated_for_tun_mode`, `dns_proxy::tests::test_dns_proxy_forwards_over_upstream_not_empty_stub`.

### GP-SEC-05 (High — CVSS 8.1): SSRF, Plaintext HTTP Downgrade, Unbounded Body Read, and URL Logging in Subscription Fetching
- **Area:** B (Untrusted Input) / C (Secrets)
- **Location:** `crates/ghostpath-daemon/src/engine.rs:843-980`
- **Description:** `import_subscription` and `refresh_subscription` create a bare `reqwest::Client::builder().timeout(15s)` without: (a) rejecting loopback (`127.0.0.0/8`, `::1`), RFC1918 private, link-local (`169.254.0.0/16` metadata), or CGNAT IPs after DNS resolution and across HTTP redirects; (b) disabling redirects or re-validating redirect targets; (c) enforcing HTTPS by default; (d) capping response size (`content-length` / streaming byte cap); (e) redacting subscription URLs (containing auth tokens) from `info!`/`warn!` logs.
- **Exploit / Failure Scenario:** A malicious `.vless`/UI input imports `http://169.254.169.254/latest/meta-data/` or redirects `https://evil.tld/sub -> http://127.0.0.1:8080/internal-admin`, forcing `root` `ghostpathd` to probe local/LAN/cloud endpoints, OOM via a 2 GB HTTP response, or leak sensitive subscription tokens into system journals (`journalctl`).
- **Fix:** Implement a dedicated `SafeHttpFetcher` that enforces `https://`, disables automatic redirects (`redirect::Policy::none()` + manual step-bounded redirect validation up to 3 hops), resolves DNS explicitly, validates every resolved `SocketAddr` is globally routable (`is_public_ip`), pins resolved socket via `resolve_to_addrs`, caps body at 4 MiB, sets connect/total timeouts, and redacts query/path tokens when logging.
- **Regression Test:** `engine::tests::test_subscription_rejects_ssrf_loopback_private_and_redirects`, `test_subscription_rejects_oversized_response_and_http`.

### GP-SEC-06 (High — CVSS 7.8): Crash Recovery (`--recover`) Neglects Routing, DNS, and KillSwitch Journal Entries
- **Area:** D (Crash Recovery) / A (Daemon Lifecycle)
- **Location:** `crates/ghostpath-daemon/src/main.rs:46-63`, `crates/ghostpath-daemon/src/engine.rs`
- **Description:** `ghostpathd --recover` loads `UndoJournal` and only invokes `GeoSpoofManager::recover_from_journal()`, then deletes `undo_journal.json`. It completely skips `RouteManager::recover()`, `DnsManager` restoration, and `KillSwitch::disable()`! Furthermore, `DaemonEngine::new()` does not automatically recover stale state after a `SIGKILL`/crash on service restart.
- **Exploit / Failure Scenario:** After `SIGKILL` or power loss while connected, `/etc/resolv.conf`, `ip rule` table `5180`, and `inet ghostpath` drop rules remain stuck on the machine; even running `ghostpathd --recover` (or `apt remove` which calls `--recover`) leaves the host without internet connectivity until manual intervention.
- **Fix:** Have `ghostpathd --recover` and `DaemonEngine::startup_recover()` cleanly revert routes (`RouteManager::recover`), DNS (`DnsManager::restore_from_journal_or_backup`), KillSwitch (`KillSwitch::disable`), stale TUN link (`ghost0`/`awg-ghost0`), and GeoSpoof state, only deleting the journal after cleanup succeeds.
- **Regression Test:** `main_recovery::tests::test_crash_recovery_restores_all_subsystems`.

### GP-SEC-07 (High — CVSS 7.8): UAPI Newline Injection and CLI Option (`-`) Injection in `AwgBackend` and `ghostpath-net`
- **Area:** A (Command Execution) / B (Untrusted Input)
- **Location:** `crates/ghostpath-daemon/src/tunnel/awg.rs`, `crates/ghostpath-net/src/coexistence.rs`, `crates/ghostpath-net/src/manager.rs`
- **Description:** `build_uapi_config` embeds `i1..i5`, `endpoint`, and `allowed_ip` verbatim into line-oriented UAPI commands. `AwgBackend`, `CoexistenceManager`, and `TunDevice` also pass `interface_name`, `address`, `cidr`, and endpoints to `Command::new("ip")` without rejecting `\n`, `\r`, `\0`, or leading `-` (CLI flag injection) and resolve binaries via mutable `PATH` (`which`).
- **Exploit / Failure Scenario:** A malicious `.conf` / `vpn://` profile sets `i1 = "<b1>\nreplace_peers=true\npublic_key=<attacker>"` to inject rogue WireGuard peers into `amneziawg-go`, or passes `-force` / malformed args to `ip`.
- **Fix:** Add strict input validators (`validate_uapi_value`, `validate_iface_name`, `validate_cidr`, `validate_ip_addr`, `validate_ip_port`, `validate_uapi_hex_key`) rejecting control chars, whitespace, leading `-`, and non-allowlisted syntax; resolve system binaries (`ip`, `nft`, `resolvectl`, `timedatectl`, `systemctl`, `amneziawg-go`) from trusted fixed system prefixes (`/usr/sbin`, `/sbin`, `/usr/bin`, `/bin`).
- **Regression Test:** `awg::tests::test_uapi_rejects_newline_injection`, `validators::tests::test_rejects_option_injection_and_bad_iface`.

### GP-SEC-08 (High — CVSS 7.4): Over-Permissive IPC Socket (`0o666`), Over-Broad `adm` Group Access, and Unbounded Socket DoS
- **Area:** A (Privilege Boundary & IPC)
- **Location:** `crates/ghostpath-daemon/src/ipc/server.rs:42-142`, `crates/ghostpath-daemon/src/ipc/auth.rs:80`
- **Description:**
  1. `UnixListener` sets socket mode `0o666`, allowing every local user (`nobody`, compromised web daemon, guest accounts) to open unlimited connections before `SO_PEERCRED` check runs inside the task.
  2. `is_user_in_allowed_groups` authorizes `adm` (Debian's passive log-reading group, not an admin group).
  3. Connection handler uses unbounded `buf_reader.read_line(&mut line)` with no byte limit, no request timeout, no protocol version field check, and unbounded `tokio::spawn` tasks.
- **Exploit / Failure Scenario:** An unprivileged local process opens 10,000 connections or streams a 4 GB single line without `\n`, crashing `root` `ghostpathd` via OOM (denial of service / killing VPN & KillSwitch). Or an `adm`-group service account takes control of VPN routing and GeoSpoof.
- **Fix:** Create runtime socket dir `/run/ghostpath` with mode `0o750` (owned `root:ghostpath`) and socket mode `0o660` (before exposing or atomically via umask/fchmod); remove `adm` from allowed groups; bound IPC frame size (e.g. `1 MiB`), enforce read/write timeouts, limit concurrent IPC connections via `tokio::sync::Semaphore` (e.g., 32), check `SO_PEERCRED` synchronously before reading payload, and add per-UID sliding-window rate limiting on expensive RPCs (`importSubscription`, `refreshSubscription`, `pingAll`).
- **Regression Test:** `ipc::server::tests::test_ipc_rejects_oversized_line_and_caps_concurrency`, `ipc::auth::tests::test_adm_group_denied`.

### GP-SEC-09 (High — CVSS 7.3): Secret Permissions Race, Non-Atomic Config Writes, `/tmp` Fallback, and Symlink/TOCTOU Risk
- **Area:** A (File Safety / TOCTOU) / C (Secrets)
- **Location:** `crates/ghostpath-core/src/config.rs:64-78`, `crates/ghostpath-core/src/paths.rs:65`, `crates/ghostpath-net/src/geo_spoof.rs`, `crates/ghostpath-net/src/dns.rs`, `crates/ghostpath-net/src/journal.rs`
- **Description:**
  1. `GhostConfig::save_to_path` calls `fs::write` (creates file `0o644` under default umask) *before* `set_permissions(0o600)`.
  2. Writes to `config.toml`, `undo_journal.json`, backups, and `/etc/geolocation` are non-atomic or do not open with `O_NOFOLLOW` + exclusive temp file + `fsync` + atomic `rename`.
  3. `dirs_fallback_config()` falls back to `/tmp/ghostpath` when `HOME` and `GHOSTPATH_CONFIG_DIR` are unset.
- **Exploit / Failure Scenario:** Local user races config creation to read keys before `chmod 0600` completes, or places a symlink at `/tmp/ghostpath/config.toml` when `HOME` is unset to make privileged process read/write attacker files.
- **Fix:** Replace `/tmp/ghostpath` fallback with safe non-shared fallback (error or XDG/user-specific private dir), implement an `atomic_write_secure(path, bytes, mode)` helper using `OpenOptions::custom_flags(O_NOFOLLOW)` on a random `tempfile` in the same `0o700` parent directory, `chmod 0o600`, `sync_all`, and atomic `rename`, verifying parent/target ownership and non-symlink status on reads/writes for config, journal, backups, and `/etc/geolocation`.
- **Regression Test:** `config::tests::test_atomic_save_rejects_symlink_and_enforces_0600`, `paths::tests::test_no_tmp_fallback_when_home_unset`.

### GP-SEC-10 (High — CVSS 7.5): Silent Direct Bypass of `.ir` Domains/IPs & Fallback to Unencrypted `socks` Outbound
- **Area:** C (Privacy / Censorship Circumvention) / D (Correctness)
- **Location:** `crates/ghostpath-proto/src/config_gen/singbox.rs:35-39,234-241`
- **Description:**
  1. Even when user configures an empty rule list (`rules: []`), `generate_singbox_config` injects `default_routing_rules()` forcing `geosite:ir` and `geoip:ir` **`direct`** outside the tunnel!
  2. When a `VLESS`/`VMess`/`Trojan`/`SS` node lacks `raw_link` or `parse_*` fails, `build_outbound` silently falls back to unencrypted `"type": "socks"` to `node.host:node.port`!
- **Exploit / Failure Scenario:** User explicitly deletes `.ir` bypass rules expecting all traffic to go through the VPN, yet `.ir` traffic still exits directly on local ISP; or malformed `raw_link` downgrades encrypted VLESS-Reality to plaintext SOCKS5.
- **Fix:** Respect empty `options.rules` (only add implicit `private` LAN direct rule if `allow_lan` is true, never silently inject `.ir` rules when empty), and return `CoreError::Config` instead of falling back to `"type": "socks"` for non-SOCKS protocols.
- **Regression Test:** `singbox::tests::test_empty_rules_does_not_inject_ir_bypass`, `singbox::tests::test_malformed_vless_fails_closed_no_socks_fallback`.

### GP-SEC-11 (Medium — CVSS 6.3): Weak `systemd` Exposure Score (`6.0 MEDIUM`) and Missing Process Dump/Umask Hardening
- **Area:** A (Daemon Privileges & Service Hardening)
- **Location:** `systemd/ghostpathd.service:1-46`, `crates/ghostpath-daemon/src/main.rs`
- **Description:** `systemd-analyze security --offline=true systemd/ghostpathd.service` reports **6.0 MEDIUM**. Missing `UMask=0077`, `ProtectKernelTunables=yes`, `ProtectKernelLogs=yes`, `ProtectHostname=yes`, `PrivateTmp=yes`, `RestrictNamespaces=yes`, `MemoryDenyWriteExecute=yes`, `SystemCallArchitectures=native`, `SystemCallFilter=@system-service`, `RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6 AF_NETLINK`, and runtime `PR_SET_DUMPABLE=0` + `RLIMIT_CORE=0`. (Note: `ProtectClock=yes` would block `timedatectl` D-Bus timezone if local; `ReadWritePaths=/etc` is required for `/etc/geolocation`, `/etc/localtime`, `/etc/resolv.conf` unless scoped tighter to `/etc/geolocation /etc/localtime /etc/timezone /etc/resolv.conf /run/ghostpath /var/lib/ghostpath`).
- **Fix:** Harden `ghostpathd` startup with `prctl(PR_SET_DUMPABLE, 0)` + `setrlimit(RLIMIT_CORE, 0)` + `umask(0o077)` in `ghostpath-daemon`, and tighten `systemd/ghostpathd.service` (permissions `0644` too—note it had world-writable/exec bits on NTFS/repo check, ensure clean `0644` in packaging) while preserving `/dev/net/tun`, `CAP_NET_ADMIN`, `CAP_NET_RAW`, `CAP_NET_BIND_SERVICE`, D-Bus, and `/etc/{geolocation,localtime,timezone,resolv.conf}`. Re-measure with `systemd-analyze security`.
- **Regression Test:** Verify `systemd-analyze security` score drop and runtime `RLIMIT_CORE == 0` / non-dumpable assertion in `daemon::tests`.

### GP-SEC-12 (Medium — CVSS 5.8): Unvalidated `updateSettings` Inputs and Secret Exposure via `ServerNode::raw_link`
- **Area:** A (IPC Validation) / C (Secrets)
- **Location:** `crates/ghostpath-daemon/src/engine.rs:715-765`, `crates/ghostpath-core/src/models.rs:72`
- **Description:** `update_settings` casts `mtu as u16` silently truncating `>65535`, accepts arbitrary `tunStack`, invalid DNS strings, invalid `excludedInterfaces` (allowing flag injection), and `socksPort`/`httpPort` of `0` or privileged ports without bounds; `listNodes` / `getStatus` serializes full `ServerNode` including `raw_link` (which contains private keys/UUIDs/passwords) back over IPC to UI/CLI.
- **Fix:** Validate `mtu` in `576..=9000`, `socks_port`/`http_port` in `1024..=65535` (and distinct), `tun_stack` in `["system", "gvisor", "mixed"]`, `dns_primary`/`dns_fallback` as valid IP/DoH/DoT endpoints without control/shell chars, `excluded_interfaces` via strict interface glob validator; redact `raw_link` when serializing `ServerNode` in IPC responses (`listNodes`/`getStatus`) while preserving `raw_link` in daemon storage.
- **Regression Test:** `engine::tests::test_update_settings_rejects_invalid_mtu_ports_and_ifaces`, `test_list_nodes_redacts_raw_link`.

### GP-SEC-13 (Medium — CVSS 5.9): Protocol Parser Input Bounds & Silent Truncation Bugs
- **Area:** B (Untrusted Input)
- **Location:** `crates/ghostpath-proto/src/amnezia/ini_parser.rs`, `crates/ghostpath-proto/src/parser/{subscription,vmess,ss,vless}.rs`
- **Description:**
  1. `ini_parser::parse_amnezia_ini` has no total byte or line count limit, enabling memory/CPU exhaustion on giant INI inputs.
  2. `subscription.rs:39,43` casts Clash/sing-box YAML/JSON `port` via `as u16` (silently wrapping `70000` -> `4464`) and defaults missing `server` to `"127.0.0.1"`.
  3. `vmess.rs`, `ss.rs`, `vless.rs`, `trojan.rs` lack input length caps, port `0` rejection, and UUID/cipher validation.
- **Fix:** Enforce max input size caps (e.g., 512 KiB per URI/INI, max 10k INI lines), validate `1..=65535` ports cleanly via `u16::try_from`, reject empty hosts instead of defaulting to `127.0.0.1`, reject control chars in hosts/remarks/UUIDs, and add property/fuzz tests.
- **Regression Test:** `parser::tests::test_rejects_port_overflow_zero_empty_host_and_oversized_ini`.

### GP-SEC-14 (Medium — CVSS 5.3): Deep Link Lacks Confirmation Dialog & Rate Limit; Tauri CSP Allows `'unsafe-inline'` & `withGlobalTauri`
- **Area:** F (Desktop App & UI) / B (Deep Links)
- **Location:** `UI/src/App.tsx:247-250`, `crates/ghostpath-gui/tauri.conf.json:10,25`, `crates/ghostpath-gui/src/main.rs`
- **Description:**
  1. Deep link (`vless://`, `vpn://`, etc.) emits `deep_link_imported` directly on second-instance spam without rate-limiting or parsing/previewing endpoint host+protocol in a strict confirmation prompt before user consent.
  2. `tauri.conf.json` has `withGlobalTauri: true` and `style-src 'self' 'unsafe-inline'`, and `main.rs` lacks explicit `on_navigation` / `on_new_window` WebView guards blocking external URLs.
- **Fix:** Add explicit deep-link preview & confirmation banner/prompt inside `AddProfileModal` (no auto-import or auto-connect, rate-limited in `single_instance.rs`); set `withGlobalTauri: false` (since `@tauri-apps/api` or typed bridge handles `window.__TAURI_INTERNALS__`—verify exact UI bridge in `UI/src/services/api.ts`), remove `'unsafe-inline'` from `style-src` if Tailwind compiles static CSS cleanly (test build!), and enforce `on_navigation` / `on_new_window` origin allowlist in Tauri.
- **Regression Test:** `single_instance::tests::test_deep_link_rate_limit_and_sanitization`, `security_tests::test_strict_csp_and_navigation_policy`.

### GP-SEC-15 (Medium — CVSS 5.0): Plaintext AWG Keys Retained Without `ZeroizeOnDrop`; Secret Leak Hygiene
- **Area:** C (Secrets & Zeroization)
- **Location:** `crates/ghostpath-daemon/src/tunnel/awg.rs:22`, `crates/ghostpath-proto/src/amnezia/model.rs`
- **Description:** `AwgBackend` stores `last_config: Option<AwgProfileConfig>` on the heap indefinitely after tunnel disconnect, and `AwgProfileConfig` does not implement `ZeroizeOnDrop`. Also verify no `raw_link`/keys/passwords appear in `Debug` or error chains.
- **Fix:** Drop `last_config` on `stop()` with `.take()` + explicit zeroization, derive/implement `Zeroize` + `ZeroizeOnDrop` on `AwgInterfaceConfig` / `AwgPeerConfig` / `AwgProfileConfig`, and mask sensitive fields in custom `Debug` impls.
- **Regression Test:** `awg::tests::test_stop_clears_last_config_and_zeroizes_keys`.

### GP-SEC-16 (Medium — CVSS 5.5): TUN FD Leak on `ioctl` Error, Ignored `ip` Exit Status, and Unwired `CoexistenceManager`
- **Area:** D (Networking Correctness)
- **Location:** `crates/ghostpath-tun/src/lib.rs:207-246`, `crates/ghostpath-daemon/src/engine.rs`
- **Description:**
  1. `open_linux_tun` binds `raw_fd`, runs `ioctl`, and returns early on error (Wait: `file` IS dropped on early error before `mem::forget(file)`, good, BUT let's wrap in `OwnedFd` right after open and also check lines 221–246 where `Command::new("ip")` return values are discarded with `let _ = ...` so MTU/IP/route failures go unnoticed!).
  2. `CoexistenceManager` (`crates/ghostpath-net/src/coexistence.rs`) is implemented and tested, yet `DaemonEngine::connect()` never invokes it to preserve `excluded_interfaces` (`docker0`, `br-*`, `virbr*`) and LAN bypasses!
- **Fix:** Check `Command::new("ip")` exit status in `TunDevice::configure_linux_interface` and `TunDevice::close`, and wire `CoexistenceManager` into `DaemonEngine::connect()`/`disconnect()` with `UndoJournal` tracking.
- **Regression Test:** `tun::tests::test_configure_interface_validates_inputs_and_propagates_errors`, `engine::tests::test_coexistence_rules_wired`.

### GP-SEC-17 (Medium — CVSS 5.2): Supply Chain Checksum Missing on `amneziawg-go` & Packaging Lifecycle Gaps
- **Area:** E (Supply Chain) / G (Packaging)
- **Location:** `packaging/build-deb.sh`, `packaging/rpm/ghostpath-vpn.spec`, `packaging/arch/PKGBUILD`, `packaging/debian/{postinst,prerm}`
- **Description:**
  1. `packaging/bin/amneziawg-go` (SHA-256 `ccb97ba58b6d4a3ab5ee44e3c969224b708c720ea74470302780e183599ad308`) is copied into packages without hash verification in `build-deb.sh`, `PKGBUILD`, or `ghostpath-vpn.spec`.
  2. `ghostpath-vpn.spec` copies `amneziawg-go` in `%install` conditionally yet omits it from `%files`; `PKGBUILD` has `sha256sums=('SKIP')`; Debian `prerm` runs stop/`--recover` unconditionally even on package upgrade rather than checking `remove`/`deconfigure` vs `upgrade`, and lacks a `postrm` purge cleanup.
- **Fix:** Add `packaging/bin/amneziawg-go.sha256` and enforce `sha256sum -c` in all package build scripts; fix `.spec` `%files` and `postinst`/`prerm`/`postrm` lifecycle handling; document source & version pin for `amneziawg-go`.
- **Regression Test:** Packaging lint & build verification checks in `scripts/verify_no_demo_content.sh` / `packaging/build-deb.sh`.

### GP-SEC-18 (Low — CVSS 3.1): Rust Hygiene (`#![forbid(unsafe_code)]`, `clippy::pedantic`, Signal `.expect()`)
- **Area:** E (Rust Code Quality)
- **Location:** `crates/*/src/{lib,main}.rs`, `crates/ghostpath-daemon/src/main.rs:106-108`
- **Description:** None of the safe crates declare `#![forbid(unsafe_code)]`; `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` fails with pedantic lints (`ghostpath-core` alone has 38 diagnostics); `main.rs:106-108` uses `.expect()` for signal registration; workspace lacks `deny.toml` and SBOM generation.
- **Fix:** Add `#![forbid(unsafe_code)]` to all crates except `ghostpath-tun` (which isolates minimal documented `unsafe` syscalls), replace `.expect()` with `?`, resolve/justify `clippy::pedantic` across the entire workspace, add `deny.toml` and reproducible SBOM script.
- **Regression Test:** `cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic` exits 0.

### GP-SEC-19 (Low — CVSS 3.3): UI Control-Char Sanitization, RTL/Persian Rendering, and DNS Default Consistency
- **Area:** F (Desktop App & UI)
- **Location:** `UI/src/App.tsx`, `UI/src/data/vpnData.ts:45-46`
- **Description:** Imported node/subscription names or daemon errors could carry ANSI/Unicode BiDi override / control characters when toasted or logged; Persian/RTL remarks need `dir="auto"` / `unicode-bidi: plaintext`; frontend initial DNS defaults (`https://1.1.1.1/dns-query`) mismatch backend `AppSettings::default()` (`1.1.1.1`).
- **Fix:** Strip control/BiDi-override chars (`\u0000-\u001F`, `\u007F`, `\u202A-\u202E`, `\u2066-\u2069`) from rendered/toasted untrusted strings, apply `dir="auto"` on user-supplied titles/remarks, and align UI/backend default DNS settings cleanly.
- **Regression Test:** Frontend unit/type check + build + bundle scan.

### GP-SEC-20 (Info): Missing Security Policy, Architecture Threat Docs, and Automated CI Security Gates
- **Area:** H (Testing, CI & Docs)
- **Location:** Root repo & `.github/workflows/desktop_ci.yml`
- **Description:** Repo lacks `SECURITY.md`, `docs/THREAT_MODEL.md` / `docs/TROUBLESHOOTING.md`, honest non-goals/limitations, fuzz harnesses, and CI jobs for `clippy::pedantic`, `cargo audit`/`deny`, secret scanning, shellcheck, and namespace integration tests.
- **Fix:** Add `SECURITY.md`, `docs/THREAT_MODEL.md`, `docs/TROUBLESHOOTING.md`, fuzz target crate/harnesses, namespace leak/crash-recovery test script, and expand `.github/workflows/desktop_ci.yml`.

---

## Independent Hostile Re-Audit Findings (GP-SEC-21 to GP-SEC-27)

During the independent hostile re-audit of the remediated codebase, the following additional Critical, High, and Medium gaps were uncovered and fixed:

### GP-SEC-21 (Critical — CVSS 9.0): `atomic_write_private` on `/etc/resolv.conf` Chmods `/etc` to `0700` & Breaks Non-Root DNS
- **Area:** A (Filesystem Safety) / D (Networking Correctness)
- **Location:** `crates/ghostpath-core/src/fs_safe.rs:80-85`, `crates/ghostpath-net/src/dns.rs:162,180,215`, `crates/ghostpath-net/src/geo_spoof.rs:293,324`
- **Description:** `atomic_write_with_mode` unconditionally called `ensure_private_dir(parent)` which executes `chmod 0700` on `parent`. Calling `atomic_write_private("/etc/resolv.conf", ...)` in `DnsManager::backup_and_write_resolv_conf()` would `chmod 0700 /etc` (locking all non-root users out of `/etc/passwd`, `/etc/hosts`, `/etc/ssl/certs`) AND write `/etc/resolv.conf` with mode `0600` (breaking DNS for every unprivileged user). Additionally, `restore_dns` / `DnsGuard::drop` used `fs::copy` (following symlinks) and `geo_spoof.rs` used `fs::write` on `/etc/geolocation` and `/etc/geoclue/conf.d/99-ghostpath.conf`.
- **Fix:** Added `fs_safe::atomic_write_system_file(path, data, mode)` which verifies `parent` is a non-symlink directory without altering `parent`'s permissions, writes atomically with `O_CREAT | O_EXCL | O_NOFOLLOW` + `0644`, and used it in `dns.rs` (both write and restore) and `geo_spoof.rs`.
- **Regression Test:** `fs_safe::tests::test_atomic_write_system_file_preserves_parent_mode`.

### GP-SEC-22 (Critical — CVSS 8.6): `ghost0` TUN Dataplane Still Disconnected in `DaemonEngine::connect()`
- **Area:** D (Networking Correctness)
- **Location:** `crates/ghostpath-daemon/src/engine.rs:270-365`, `crates/ghostpath-proto/src/outbound/managed.rs:107-115`
- **Description:** Despite `singbox.rs` supporting `tun-in` and `ManagedCoreOutbound` having `.with_tun()`, `DaemonEngine::connect()` still opened `TunDevice::new("ghost0")` separately and never called `.with_tun("ghost0", ...)` on `ManagedCoreOutbound`, leaving `ghost0` with no packet forwarder. Moreover, `CoreEngine::XrayCore` passed `sing-box` JSON to `xray`.
- **Fix:** Wired `.with_tun("ghost0", config.settings.mtu, &config.settings.tun_stack)` into `ManagedCoreOutbound` in `DaemonEngine::connect()`, starting `sing-box` after `KillSwitch::enable` so `sing-box` owns `ghost0` before `RouteManager` and `DnsManager` attach; rejected `CoreEngine::XrayCore` with `tun_interface.is_some()` cleanly. Also fixed a flaky env-var race in `managed.rs` tests by adding `with_config_dir()`.
- **Regression Test:** `engine::tests::test_connect_fails_closed_when_core_or_tun_unavailable`, `managed::tests::test_managed_outbound_start_and_ping`.

### GP-SEC-23 (High — CVSS 7.6): IPv6 Leak When `ipv6_routing == false`, Echo-Stub `DnsProxy`, and `resolvectl` URL Rejection
- **Area:** C (Leak Prevention) / D (Networking Correctness)
- **Location:** `crates/ghostpath-net/src/manager.rs:82-98`, `crates/ghostpath-net/src/dns_proxy.rs`, `crates/ghostpath-net/src/dns.rs`
- **Description:**
  1. When `ipv6_routing == false` and `kill_switch == false`, `RouteManager` installed zero IPv6 rules, allowing IPv6 traffic to bypass the tunnel.
  2. `DnsProxy` was still a fake echo responder returning 0-answer `NOERROR` packets.
  3. `DnsManager` passed `https://...`/`tls://...` URLs verbatim to `resolvectl dns` and `/etc/resolv.conf`.
- **Fix:** Added IPv6 `blackhole default` routing in table `5180` when `ipv6_enabled == false`; upgraded `DnsProxy` to a real length-prefixed DNS-over-TCP/SOCKS5 forwarder returning `SERVFAIL` on upstream failure; added `resolve_dns_ip()` in `dns.rs`.
- **Regression Test:** `dns_proxy::tests::test_dns_proxy_forwards_over_upstream_and_returns_servfail_when_down`.

### GP-SEC-24 (High — CVSS 7.3): `IpcServer` Idle-Connection DoS, Socket Dir Symlink Check, and Per-UID Rate Limiting
- **Area:** A (Privilege Boundary & IPC)
- **Location:** `crates/ghostpath-daemon/src/ipc/server.rs`
- **Description:** `handle_client` lacked a read timeout on `fill_buf()`, allowing 32 idle connections to permanently exhaust `MAX_CONCURRENT_IPC_CLIENTS`; `run()` did not check `socket_path.parent()` for symlinks; and expensive RPCs lacked per-UID rate limiting.
- **Fix:** Added symlink check + `0755` enforcement on socket parent dir, 30s `IPC_IDLE_READ_TIMEOUT` on `fill_buf()`, and `UidRateLimiter` (10 expensive RPCs per 10s window per UID).
- **Regression Test:** `ipc::server::tests::test_ipc_rate_limiter_bounds_expensive_rpcs`.

### GP-SEC-25 (High — CVSS 7.1): `PATH`-Resolved Privileged Binaries & Unsanitized/Unredacted `append_log` Entries
- **Area:** A (Command Execution) / C (Secrets)
- **Location:** `crates/ghostpath-core/src/paths.rs`, `crates/ghostpath-net/src/*.rs`, `crates/ghostpath-tun/src/lib.rs`, `crates/ghostpath-daemon/src/{tunnel/awg.rs, engine.rs}`
- **Description:** `Command::new("ip")`, `"nft"`, `"resolvectl"`, `"systemctl"` relied on `PATH`; `DaemonEngine::connect()` logged `node.host` in `append_log`, and `import_profile`/`import_subscription` logged unsanitized profile/server names.
- **Fix:** Added `GhostPaths::resolve_trusted_system_bin()` (`/usr/sbin`, `/sbin`, `/usr/bin`, `/bin`) across all privileged command invocations; redacted `node.host` and applied `sanitize_display_text(..., 64)` in `append_log`.

### GP-SEC-26 (Medium — CVSS 5.4): Unenforced `MAX_SUBSCRIPTION_LINES`, Missing `SystemCallFilter`, Deep-Link Toast Secret Leak
- **Area:** B / A / F
- **Location:** `crates/ghostpath-proto/src/parser/subscription.rs:104`, `systemd/ghostpathd.service`, `UI/src/App.tsx:247`
- **Description:** `parse_uri_list` ignored `MAX_SUBSCRIPTION_LINES`; `ghostpathd.service` omitted `SystemCallFilter=@system-service`; `App.tsx` toasted the first 30 chars of deep links (exposing UUIDs/tokens).
- **Fix:** Enforced `.take(MAX_SUBSCRIPTION_LINES)` in `parse_uri_list`; added `SystemCallFilter=@system-service` (reducing `systemd-analyze security` exposure from `4.2 OK` to `2.8 OK`); redacted deep-link toast to scheme only; updated CI workflow with `cargo audit`, `cargo deny`, `shellcheck`, `systemd-analyze security`.
- **Regression Test:** `parser::subscription::tests::test_subscription_enforces_max_lines`.

### GP-SEC-27 (Medium — CVSS 4.8): RPM Spec `%files` Missing `amneziawg-go` & Bogus Changelog Date
- **Area:** G (Packaging)
- **Location:** `packaging/rpm/ghostpath-vpn.spec`
- **Description:** `ghostpath-vpn.spec` installed `amneziawg-go` without listing it in `%files` and had `Sun Nov 23 2026` (Nov 23, 2026 is Monday), causing an `rpmlint` warning.
- **Fix:** Added `%{_bindir}/amneziawg-go` to `%files` and corrected changelog date to `Mon Nov 23 2026`.
