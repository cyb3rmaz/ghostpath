# Architectural and Design Decisions (DECISIONS.md)

This document records the architectural, network, and security decisions for **GhostPath VPN**. It is maintained across all phases as required by `.agents/rules/superpowers.md`.

---

## 1. Product Naming & Linux Identifiers

- **Date:** 2026-11-23
- **Status:** Approved
- **Identifiers:**
  - **Product Name:** GhostPath VPN
  - **Cargo Crates:** `ghostpath-core`, `ghostpath-proto`, `ghostpath-tun`, `ghostpath-net`, `ghostpath-daemon`, `ghostpath-cli`, `ghostpath-gui`
  - **CLI Binary:** `ghostpath`
  - **Daemon Binary & Systemd Unit:** `ghostpathd` / `ghostpathd.service`
  - **Config Directory:** `~/.config/ghostpath` (mode `0700` / `0600` for files)
  - **System State & Journal Directory:** `/var/lib/ghostpath` and `/run/ghostpath`
  - **TUN Interface:** `ghost0` (IPv4: `10.8.0.2/24`, peer `10.8.0.1`; IPv6: `fd00:5180::2/64`, peer `fd00:5180::1`)
  - **TUN MTU:** `1420` (prevents packet fragmentation over PPPoE, 4G/LTE mobile networks, and filtered ISP middleboxes in Iran); TCP MSS clamped to `1380` (`MTU - 40`) for IPv4 and `1360` (`MTU - 60`) for IPv6.
  - **Fwmark:** `0x4750` (`18256` decimal, derived from ASCII `'G''P'`). Mask: `0x4750` or `0xffffffff`.
  - **Routing Table ID:** `5180` (named `ghostpath`)
  - **IP Rule Priorities:**
    - `5180`: `from all fwmark 0x4750 lookup main` (bypass rule for proxy core outbound sockets to avoid routing loops)
    - `5181`: `from all lookup 5180` (or split default routing rules `0.0.0.0/1` and `128.0.0.0/1` in table `5180`)
    - Leaves priorities `< 5180` and `> 5181` open for other VPNs (e.g. WireGuard rule `51820`, Tailscale, local network rules).
  - **nftables Table:** `ghostpath` (family `inet`)
  - **IPC Socket:** `/run/ghostpath/ghostpathd.sock` (mode `0660`, group `ghostpath` or owner root)

---

## 2. Core Backend Strategy: Trait Abstraction with Managed External Core + Native Fallback

- **Date:** 2026-11-23
- **Status:** Approved
- **Context:**
  Section 4 of the specification requires evaluating native Rust outbound implementations vs a managed external core backend (`sing-box` or `Xray-core` as a supervised child process) under a clean abstraction (`trait Outbound`).
- **Options Evaluated:**
  - *Option A: Pure Native Rust implementations.*
    - Pros: Single binary distribution, zero external dependencies.
    - Cons: Censorship circumvention in Iran relies on complex, rapidly changing obfuscation techniques: VLESS XTLS Vision (eliminating TLS-in-TLS signatures), Reality (mimicking Chrome/Firefox TLS 1.3 ClientHello handshakes to external destination SNIs without possessing private keys), uTLS fingerprints, and TLS packet fragmentation. Rust crates for Reality and XTLS Vision are either experimental, unmaintained, or incomplete. Implementing and maintaining these in pure Rust presents high security and anti-censorship risks.
  - *Option B: Clean `trait Outbound` abstraction with managed supervised core (`sing-box` primary, `xray-core` optional) and native Rust fallback.*
    - Pros: Battle-tested censorship circumvention. `sing-box` is actively maintained with world-class support for Reality, Vision, uTLS fingerprints, and gRPC/WS multi-pathing. GhostPath's Rust daemon supervises the process (lifecycle, config generation, health checks, SOCKS5/direct multiplexing), while GhostPath owns all Linux system integration (TUN device `ghost0`, routing tables, `ip rule`, nftables, DNS resolution, systemd, and IPC). The frontend UI already includes a toggle between `sing-box` and `xray-core`.
- **Decision:**
  Adopt **Option B**. Define `trait Outbound` in `ghostpath-proto`. Implement `ManagedCoreOutbound` (supporting `sing-box` as default and `xray-core` as alternative) and a native Rust fallback `NativeOutbound` (for SOCKS5, HTTP, Shadowsocks, WireGuard).

---

## 3. UI Integration Method

- **Date:** 2026-11-23
- **Status:** Approved
- **Context:**
  The UI exists in `UI/` (React 19 + TypeScript + Tailwind CSS v4 + Vite).
- **Decision:**
  - Adopt **Tauri 2** with WebKitGTK in `ghostpath-gui` to turn GhostPath VPN into a native Linux desktop application.
  - **No local HTTP server or browser dependency:** The desktop app is fully offline and self-contained; the UI does not make HTTP requests to `127.0.0.1` or any remote address.
  - **Direct Unix Socket IPC:** Rust backend connects to `ghostpathd` at `/run/ghostpath/ghostpathd.sock` using unprivileged user credentials (with `ghostpath` group permissions), handling commands via Tauri `invoke` and real-time push events (`state_changed`, `traffic_updated`, `log_appended`) via Tauri window events.
  - **Desktop Integration:** Native system tray with menu, single-instance window focusing, XDG autostart on login, desktop notifications, and deep link URI handler (`vless://`, `vmess://`, etc.).
  - **Offline Security & CSP:** Strict CSP disabling remote scripts, styles, and fonts. Devtools disabled in release builds. All typography and assets bundled locally.
  - **Rejected Alternatives:**
    - *Electron:* Rejected due to ~150MB+ bundle size, redundant Chromium runtime, higher RAM usage, and unnecessary attack surface.
    - *Raw `wry`/`tao`:* Evaluated as fallback; Tauri 2 provides superior capabilities, plugin ecosystem (single-instance, notifications, deep link), and security sandboxing out of the box with zero runtime overhead over raw WebKitGTK.
  - The UI connects to `ghostpathd` via a versioned JSON-RPC API contract defined in `docs/API.md` over Tauri IPC and the authenticated Unix domain socket (`/run/ghostpath/ghostpathd.sock`).
  - Identified UI adjustments:
    - Replace hardcoded `tun0` text and logs with `ghost0`.
    - Document UI gaps (Chain mode selector, Excluded interfaces manager, full Doctor diagnostics view) in `docs/UI_INTEGRATION.md`.

---

## 4. Multi-VPN Coexistence and Chaining

- **Date:** 2026-11-23
- **Status:** Approved
- **Context:**
  Other VPN clients (WireGuard, OpenVPN, Tailscale) must connect *over* GhostPath without breaking or experiencing routing loops.
- **Decision:**
  - **Mode A (Other VPN over GhostPath):** GhostPath manages `ghost0` using dedicated routing table `5180` and rule priority `5181`. When a secondary VPN (e.g. WireGuard `wg0`) starts, its handshake packets to its endpoint route through `ghost0` (inheriting GhostPath's censorship-resistant tunnel). To prevent conflicts, GhostPath never flushes main routing tables or foreign rules.
  - **Mode B (GhostPath over other VPN):** `ghostpathd` supports binding the outbound core socket to a specific network interface (`SO_BINDTODEVICE`, e.g. `wg0`), enabling chaining in reverse.
  - **Excluded Interfaces & CIDRs:** GhostPath explicitly ignores and excludes interfaces matching configurable patterns (`docker0`, `br-*`, `virbr*`, `wg*`, `tailscale*`) and local LAN subnets (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `127.0.0.0/8`).

---

## 5. DNS Management Strategy

- **Date:** 2026-11-23
- **Status:** Approved
- **Context:**
  Linux systems have diverse and conflicting DNS stacks (systemd-resolved, NetworkManager, direct `/etc/resolv.conf`). DNS queries to local ISP resolvers result in DNS poisoning and severe filtering.
- **Decision:**
  - **Primary (systemd-resolved):** When `systemd-resolved` is active, GhostPath connects over D-Bus (`zbus`) to `org.freedesktop.resolve1` and calls `SetLinkDNS` and `SetLinkDomains` (with routing domain `"~."`) on interface `ghost0`. This marks `ghost0` as the highest priority resolver for all queries.
  - **Secondary (Direct `/etc/resolv.conf`):** If systemd-resolved is absent, GhostPath creates an atomic backup `/var/lib/ghostpath/resolv.conf.backup` and updates `/etc/resolv.conf` with a localhost resolver or tunnel DNS, guaranteed to restore on exit via RAII and signal handling.
  - **Internal DNS Hijack:** All port 53/853 traffic entering `ghost0` is redirected to GhostPath's internal DNS proxy which forwards queries over encrypted DoH through the proxy tunnel, preventing any DNS leaks.

---

## 6. Safety, Crash Recovery & Undo Journal

- **Date:** 2026-11-23
- **Status:** Approved
- **Context:**
  System network configuration changes must never leave the machine disconnected if the daemon crashes or is killed with `SIGKILL`.
- **Decision:**
  - **Undo Journal:** Before applying any rule, route, or nftables chain, GhostPath writes an atomic transaction record to `/var/lib/ghostpath/undo_journal.json`.
  - **Startup Recovery:** Upon launch, `ghostpathd` checks for an existing undo journal. If found (indicating a previous unclean exit or `SIGKILL`), it rolls back all recorded changes before initializing fresh state.
  - **Signal Handling & RAII:** `SIGINT`, `SIGTERM`, and `SIGHUP` invoke graceful teardown. RAII guards in Rust ensure unwinding clears active routes and interfaces.
  - **Strict Object Ownership:** Only rules with priority `5180`/`5181`, fwmark `0x4750`, interface `ghost0`, and table `ghostpath` are created or modified. Never flush entire tables.

---

## 7. Tauri 2 Desktop Shell & Zero-Network Offline Architecture

- **Date:** 2026-11-23
- **Status:** Approved
- **Context:**
  The GhostPath web UI previously required a local web server / browser. The application must run as a native, installable Linux desktop app with tray support, offline assets, and strict security boundaries.
- **Decision:**
  - **Framework:** Tauri 2 with WebKitGTK (`webkit2gtk-4.1`). Uses the system-provided WebView, resulting in an ~8.8MB AppImage and ~6.3MB Debian package (versus 150MB+ for Electron).
  - **Zero-Network UI:** The UI does not listen on or connect to `127.0.0.1`, `localhost`, or any network port. All frontend communication is bridged via Tauri IPC `invoke` commands and push events.
  - **Local Typography & Assets:** All external Google Fonts CDN links are completely removed. Fonts (`Plus Jakarta Sans`, `DM Sans`, `JetBrains Mono`, `Vazirmatn` Persian) are bundled locally via npm `@fontsource` packages.
  - **Strict Content Security Policy (CSP):**
    `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; img-src 'self' data:; connect-src 'none'`
    Blocks any outbound network requests from the webview context.
  - **Unprivileged Execution:** The GUI process runs as the standard user, never as `root`. Privileged operations (TUN allocation, routing, nftables, DNS proxy) remain strictly encapsulated in `ghostpathd`.

---

## 8. Flatpak Sandboxing Feasibility & Packaging Strategy

- **Date:** 2026-11-23
- **Status:** Approved
- **Context:**
  Evaluate shipping GhostPath as a Flatpak on Flathub.
- **Evaluation & Technical Blockers:**
  - Flatpak containers run in unprivileged user namespaces (`bubblewrap`) with isolated network namespaces or restricted permissions.
  - A Flatpak application **cannot**:
    1. Create kernel TUN interfaces (`/dev/net/tun`) without elevated host capabilities.
    2. Manipulate host routing tables (`ip route`, `ip rule`).
    3. Modify host `nftables` or `iptables` firewall rules.
    4. Directly manage systemd services on the host (`systemctl enable --now ghostpathd`).
  - While Flatpak supports `--filesystem=/run/ghostpath` to talk to a pre-existing host daemon over a Unix socket, it **cannot self-install or manage the daemon**. Packaging only the GUI in Flatpak leaves users with a non-functional app unless they also install `ghostpathd` via `.deb`, `.rpm`, or AUR on the host.
- **Decision:**
  - Do **not** distribute GhostPath VPN primarily as a standalone Flatpak at this time.
  - Support native Linux packages that bundle both the unprivileged GUI and the root systemd daemon together:
    - **Debian / Ubuntu:** `.deb` package with postinst system group and systemd service registration.
    - **Fedora / RHEL:** `.rpm` package via RPM spec.
    - **Arch Linux:** `PKGBUILD` for AUR.
    - **AppImage:** Standalone portable GUI executable with built-in first-run diagnostics that detects missing host daemon and provides copyable one-line installation commands.

---

## 9. AmneziaVPN & AmneziaWG Protocol Support and Runtime Strategy

- **Date:** 2026-11-24
- **Status:** Approved
- **Context:**
  Users need the ability to import and connect with AmneziaVPN configurations (especially AmneziaWG for censorship-resistant tunneling) inside GhostPath with full TUN mode, policy routing, DNS protection, kill switch, and multi-VPN coexistence.
- **Specifications & Verified Standards:**
  - **`vpn://` URI & `.vpn` Files:** Encoded with Base64URL wrapping Qt's `qCompress` format (4-byte big-endian uncompressed length followed by a zlib compressed JSON stream). Bounded to 2 MB maximum uncompressed size to prevent zip-bomb attacks.
  - **AmneziaWG Obfuscation Fields:**
    - v1.0: `Jc` (junk count), `Jmin`/`Jmax` (junk byte sizes), `S1`/`S2` (handshake init/response padding), `H1`–`H4` (message type header overrides).
    - v2.0+: `S3`/`S4` (cookie/transport padding), `I1`–`I5` (custom signature packets).
    - WireGuard fallback: Plain WireGuard configs are supported under the same parser with obfuscation fields zeroed/omitted.
- **Runtime Engine Evaluation:**
  - *Option A: Userspace `amneziawg-go`.* Portable across 100% of Linux distributions, MIT licensed, requires zero kernel compilation or DKMS, runs with `CAP_NET_ADMIN` on `/dev/net/tun`, controlled via standard WireGuard UAPI.
  - *Option B: Out-of-tree kernel module `amneziawg-linux-kernel-module`.* High throughput, but GPLv2, requires kernel headers, breaks on kernel updates, and fails on Secure Boot systems without custom machine owner keys (MOK).
  - *Option C: Native Rust implementation.* No production crates exist for AmneziaWG packet obfuscation algorithms.
- **Decision:**
  - Adopt **Option A (`amneziawg-go`)** as the primary default runtime engine, bundled directly inside GhostPath packages (`.deb`, `.rpm`, `AppImage`, `PKGBUILD`).
  - Introduce `TunnelBackend` trait in `ghostpath-daemon`/`ghostpath-proto` alongside the proxy `Outbound` trait to natively manage L3 WireGuard/AmneziaWG interfaces.
  - Apply socket fwmark `0x4750` on `amneziawg-go`'s UDP outbound socket to route via physical `table main` while routing system traffic into `table 5180`.
  - In `vpn://` multi-container payloads: import all supported containers (`amnezia-awg`, `amnezia-wireguard`, `amnezia-xray`, `amnezia-shadowsocks`), and report clear, honest feedback for unsupported containers (`amnezia-openvpn-cloak`, `amnezia-openvpn`, `amnezia-ipsec`).

---

## 10. System Geo Location & Timezone Spoofing Architecture

- **Date:** 2026-11-25
- **Status:** Approved
- **Context:**
  When routing traffic through a VPN server in another country, mismatches between the exit IP geolocation, the Linux OS timezone (`/etc/localtime`), and desktop/browser Geolocation API (`GeoClue2`) expose the user's true region via browser and system fingerprinting. Users also expect to pick a target country (or auto-match the connected VPN node's country) directly inside the Routing tab.
- **Options Evaluated:**
  - *UI placement:* Add a dedicated **Geo Location & System Timezone Spoofing** control panel inside the Routing view above **Split Tunneling & Rules**, while fixing dark-mode `<select>` contrast issues.
  - *System Timezone mechanism:* Use `timedatectl set-timezone <IANA_TZ>` via `systemd-timedated`, with atomic symlink fallback on `/etc/localtime` -> `/usr/share/zoneinfo/<IANA_TZ>` and `/etc/timezone` on minimal systems without `systemd-timedated`. All writes are validated against an allowlist of safe zoneinfo paths and verified country presets.
  - *System Geo Location mechanism:* Manage GeoClue2 static location using `/etc/geolocation` (4-line lat/lon/alt/accuracy specification) and a reversible drop-in `/etc/geoclue/conf.d/99-ghostpath.conf` enabling `[static-source]` and suppressing conflicting Wi-Fi/3G/IP sources while spoofing is active, then restarting `geoclue` if running.
  - *Safety & Crash Recovery:* Record `ModifiedTimezone` and `ModifiedGeolocation` actions in `UndoJournal` (`/var/lib/ghostpath/undo_journal.json`) and save pre-existing backups under `/var/lib/ghostpath/`. RAII guards and `ghostpathd --recover` guarantee full restoration of original system timezone and GeoClue state on disconnect, quit, `SIGTERM`, or crash recovery.
- **Decision:**
  - Implement a shared `GeoCountryPreset` catalog and `GeoSpoofSettings` model in `ghostpath-core`.
  - Implement reversible `GeoSpoofManager` + `GeoSpoofGuard` in `ghostpath-net` with full `--dry-run` support and `UndoJournal` integration.
  - Enforce `ProtectSystem=strict` in `systemd/ghostpathd.service` while scoping `ReadWritePaths` explicitly for `/etc/localtime`, `/etc/timezone`, `/etc/geolocation`, and `/etc/geoclue/conf.d`.
  - Provide both automatic match-on-connect (including `match-node` to follow the active VPN server's country) and manual Apply / Restore controls, plus optional strict GeoIP/GeoSite routing alignment.
