# GhostPath VPN — Residual Risk Register

This document records the remaining known risks, environment limitations, and deferred items after the security audit and remediation pass.

## 1. Environment & Tooling Limitations (Not Verified in This Session)

1. **Live Kernel Leak & Soak Testing (`tun` + `nftables` + `systemd-resolved`):**
   - WSL2's unprivileged container/user-namespace environment does not permit `CAP_NET_ADMIN` operations against host `/dev/net/tun` or `nftables` without root/VM isolation.
   - Policy routing, DNS leak behavior under sleep/resume, and multi-hour reconnect-storm soak tests should be executed in a dedicated Ubuntu 24.04 / Fedora / Arch VM before public release.
2. **Packaging Linters (`lintian`, `rpmlint`, `shellcheck`):**
   - `lintian` and `shellcheck` were not pre-installed in the WSL environment and `sudo apt` required an interactive password. Run `shellcheck packaging/*.sh scripts/*.sh` and `lintian dist/ghostpath-vpn_0.1.0_amd64.deb` in CI.
3. **Coverage-Guided Fuzzing (`cargo-fuzz` / libFuzzer):**
   - `proptest` fuzzing (`amnezia_proptest`) was run and passed, but dedicated `cargo-fuzz` libFuzzer targets with persisted corpora were not executed.

## 2. Technical Residual Risks

1. **`DnsProxy` (`crates/ghostpath-net/src/dns_proxy.rs`) — Medium/High:**
   - `DnsProxy` on `127.0.0.1:10853` is currently a minimal UDP responder used for tests and is no longer started by `DaemonEngine::connect()`. Instead, `DnsManager` configures `resolvectl` (`~.` exclusive routing domain on `ghost0`/`awg-ghost0`) or `/etc/resolv.conf` directly. If a user configures `https://...` or `tls://...` in `dnsPrimary`, `resolvectl` may reject it unless using a local DoH forwarder.
2. **Config Storage at Rest (`~/.config` / `/var/lib/ghostpath/config/config.json`) — Medium:**
   - Profile `raw_link` values (containing UUIDs, Reality keys, WireGuard private keys) and subscription URLs are stored in JSON with `0600` permissions via `atomic_write_private()`, rather than encrypted in the Linux Secret Service / kernel keyring.
3. **Tauri Global API & Style CSP (`crates/ghostpath-gui/tauri.conf.json`) — Low:**
   - `withGlobalTauri: true` and `style-src 'self' 'unsafe-inline'` remain enabled because `UI/src/services/api.ts` accesses `window.__TAURI__` and bundled styles use inline style attributes. Migrating `api.ts` to `@tauri-apps/api` imports will allow disabling `withGlobalTauri`.
4. **Systemd `ReadWritePaths` Scope — Low:**
   - `systemd/ghostpathd.service` was narrowed from `/etc` to `-/etc/geolocation -/etc/geoclue -/etc/resolv.conf -/etc/localtime -/etc/timezone`. On systems where `/etc/localtime` or `/etc/resolv.conf` is a symlink that needs replacement rather than `timedatectl`/`resolvectl` D-Bus calls, fallback symlink replacement may require `/etc` directory write access.
5. **Transitive Unmaintained Crates (`cargo audit`) — Info:**
   - `cargo audit` reported 0 CVEs/vulnerabilities, with `unmaintained` advisories on transitive dependencies (`chrono`, `proc-macro-error`, `proc-macro-error2`, `zbus_names`, `zvariant`, `zvariant_derive`) pulled in by Tauri/zbus ecosystems.

## 3. Operational Limitations (Inherent to Threat Model)

- GhostPath VPN cannot protect against browser fingerprinting, WebRTC/HTML5 geolocation prompts in browsers with independent location services, global passive ISP traffic correlation, or compromise of the remote VPN endpoint.
