# GhostPath VPN — Residual Risk Register

This document records the remaining known risks, environment limitations, and deferred items after the independent security audit and remediation pass.

## 1. Environment & Tooling Limitations (Not Verified in This Session)

1. **Live Kernel Leak & Soak Testing (`tun` + `nftables` + `systemd-resolved`):**
   - WSL2's unprivileged environment does not permit `CAP_NET_ADMIN` operations against host `/dev/net/tun` or `nftables` without root/VM isolation.
   - Policy routing, DNS leak behavior under sleep/resume, and multi-hour reconnect-storm soak tests should be executed in a dedicated Ubuntu 24.04 / Fedora / Arch VM before wide public release.
2. **Clean-VM Multi-Distro Package Lifecycle Tests:**
   - `shellcheck` (`RC=0`), `lintian` (`RC=0`), `rpmlint` (`0 errors`), and `systemd-analyze security` (`2.8 OK`) were executed in WSL2 Ubuntu; clean-VM install/upgrade/uninstall tests on Ubuntu 22.04/24.04, Fedora, and Arch with NetworkManager + systemd-resolved + Docker active remain to be executed.
3. **Coverage-Guided Fuzzing (`cargo-fuzz` / libFuzzer) & Sanitizers (`miri` / TSan / ASan):**
   - `proptest` fuzzing (`amnezia_proptest`) was run and passed; `cargo-fuzz` is not installed in this environment, and `miri`/TSan/ASan runs on nightly remain to be added.

## 2. Technical Residual Risks

1. **Config Storage at Rest (`~/.config` / `/var/lib/ghostpath/config/config.toml`) — Medium:**
   - Profile `raw_link` values (containing UUIDs, Reality keys, WireGuard private keys) and subscription URLs are stored in TOML/JSON with `0600` permissions via `atomic_write_private()`, rather than encrypted in the Linux Secret Service / kernel keyring.
2. **Tauri Global API & Style CSP (`crates/ghostpath-gui/tauri.conf.json`) — Low:**
   - `withGlobalTauri: true` and `style-src 'self' 'unsafe-inline'` remain enabled because `UI/src/services/api.ts` accesses `window.__TAURI__` and bundled styles use inline style attributes. Migrating `api.ts` to `@tauri-apps/api` imports will allow disabling `withGlobalTauri`.
3. **Systemd `ReadWritePaths` Scope — Low:**
   - `systemd/ghostpathd.service` scopes writes to `-/etc/geolocation -/etc/geoclue -/etc/resolv.conf -/etc/localtime -/etc/timezone`. On systems where `/etc/localtime` or `/etc/resolv.conf` is a symlink that needs replacement rather than `timedatectl`/`resolvectl` D-Bus calls, fallback symlink replacement is intentionally blocked by `O_NOFOLLOW` to prevent symlink-following attacks.
4. **`clippy::pedantic` Documentation Lints — Info:**
   - `cargo clippy --workspace --all-targets -- -D warnings` passes cleanly; `-W clippy::pedantic` flags `missing_errors_doc` and `doc_markdown` in `ghostpath-core`.
5. **Transitive Advisories (`cargo audit`) — Info:**
   - `cargo audit` reported 0 CVEs across workspace crates, with `unmaintained` warnings on `zvariant`/`zvariant_derive` and an upstream `unsound` advisory (`RUSTSEC-2024-0429`) on transitive `glib 0.18.5` pulled in by Tauri/WebKitGTK.

## 3. Operational Limitations (Inherent to Threat Model)

- GhostPath VPN cannot protect against browser fingerprinting, WebRTC/HTML5 geolocation prompts in browsers with independent location services, global passive ISP traffic correlation, or compromise of the remote VPN endpoint.
