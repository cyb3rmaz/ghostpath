# GhostPath VPN 🛡️

[![CI](https://img.shields.io/badge/CI-Passing-brightgreen.svg)](#testing)
[![Linux Only](https://img.shields.io/badge/Platform-Linux%20x86__64%20%7C%20aarch64-blue.svg)](#requirements)
[![Rust Edition](https://img.shields.io/badge/Rust-2021%20(1.75%2B)-orange.svg)](#building-from-source)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-lightgrey.svg)](#license)

**GhostPath VPN** is a high-performance, native Linux VPN client written in Rust, engineered specifically to provide **bulletproof, reliable TUN mode** under severe and hostile internet filtering conditions (such as the national censorship infrastructure in Iran).

Unlike conventional VPN clients that suffer from routing loops, DNS leaks, and systemd-resolved conflicts on Linux, GhostPath VPN employs kernel-level policy routing, socket fwmark exclusion, strict encrypted DNS tunneling, robust multi-VPN coexistence, and full support for **AmneziaVPN / AmneziaWG obfuscated tunnels**.

---

## 🎯 Key Problems Solved

### 1. Eliminating Linux TUN Routing Loops
* **The Root Cause:** In standard TUN clients, the proxy core's outbound socket to the remote server gets captured by the newly installed default route (`0.0.0.0/0` via `ghost0`), causing an immediate routing loop and connection freeze. Traditional `/32` host routes via the old default gateway break instantly when servers use Cloudflare/CDN endpoints or dynamic IP pools.
* **GhostPath's Solution:** Policy routing using `SO_MARK`/fwmark (`0x4750` / `18256`). All proxy outbound sockets are tagged with fwmark `0x4750`.
  - Kernel Rule `5180`: `from all fwmark 0x4750 lookup main` (bypasses tunnel completely).
  - Kernel Rule `5181`: `from all not fwmark 0x4750 lookup 5180` (captures all system traffic into table `5180`).
  - No static `/32` routes required. Works reliably with CDN, Reality, and domain-fronted endpoints.

### 2. DNS Leak & Poisoning Elimination
* **The Problem:** Linux distributions running `systemd-resolved`, `NetworkManager`, and `/etc/resolv.conf` frequently fight each other, leaking plaintext UDP queries to the local ISP resolver where they are intercepted or poisoned.
* **GhostPath's Solution:** Native integration with `systemd-resolved` over `resolvectl` using routing domain `"~."` on interface `ghost0` (or `awg-ghost0`), forcing DNS resolution into the tunnel. Includes a local DNS-over-TCP/SOCKS5 forwarder (`DnsProxy`) and an atomic `0644` fallback for `/etc/resolv.conf`.

### 3. Multi-VPN Coexistence & Chaining
* **The Problem:** Running WireGuard, OpenVPN, or Tailscale simultaneously with other clients results in routing table clashes and broken tunnels.
* **GhostPath's Solution:** Dedicated routing table (`5180`) and priority spacing (`5180`/`5181`) leave room for external VPN interfaces. External VPN traffic can ride *inside* GhostPath (WireGuard-over-GhostPath chain mode) with zero collisions.

### 4. Zero-Leak Crash Safety & Atomic Recovery
* **The Problem:** If a VPN daemon crashes or receives `SIGKILL`, leftover routes and altered DNS leave the user with dead internet.
* **GhostPath's Solution:** Atomic `UndoJournal` (`/var/lib/ghostpath/undo_journal.json`). On any exit, signal (`SIGTERM`, `SIGINT`), or crash, `ghostpathd --recover` automatically rolls back all routing rules, routes, and DNS configurations.

### 5. Strict nftables Kill Switch
* Dedicated `inet` table `ghostpath` drops any non-local, non-tunnel traffic if the connection drops unexpectedly, while preserving LAN connectivity and DHCP leases.

### 6. AmneziaVPN & AmneziaWG Obfuscated WireGuard
* **The Problem:** Plain WireGuard handshakes are instantly identified and severed by state-level Deep Packet Inspection (DPI) due to distinct packet headers, fixed message lengths, and signature bytes.
* **GhostPath's Solution:** Native support for AmneziaWG obfuscated protocol formats and parameters:
  - **Obfuscation Parameters:** Junk packet count (`Jc`), junk byte size ranges (`Jmin`..`Jmax`), handshake init/response padding (`S1`..`S4`), and message type header overrides (`H1`..`H4`).
  - **Decoders:** Bounded decompression (maximum 2 MB limit to eliminate zip bombs) for Qt `qCompress` encoded `vpn://` URI schemes, multi-container payloads, `.conf`, and `.vpn` configuration files.
  - **Engine Runtime:** Supervised userspace `amneziawg-go` engine with outbound socket fwmark `0x4750` for zero-leak routing via `awg-ghost0`, plus optional kernel module detection.
  - **Cryptographic Zeroization:** Private keys and preshared keys are protected in memory with `Zeroize` and erased upon drop. Sensitive keys are masked in UI and omitted from logs.

### 7. System Geo Location & Timezone Spoofing
* **The Problem:** Websites and desktop apps cross-check browser/OS timezones and GeoClue2 location APIs against your VPN exit IP to detect proxy usage.
* **GhostPath's Solution:** Choose any country manually or auto-match your active VPN server inside the **Routing** view or CLI (`ghostpath geo set DE`):
  - **System Timezone:** Synchronizes Linux OS timezone (`timedatectl` / `/etc/localtime`) to the selected country's IANA timezone.
  - **Desktop GeoClue2 Location:** Writes reversible static coordinates (`/etc/geolocation` & drop-in override) so location-aware apps and browsers align with the target country.
  - **Guaranteed Auto-Restore:** Original timezone and GeoClue state are backed up into `UndoJournal` and automatically restored upon disconnect, shutdown, or `ghostpathd --recover`.

---

## 🏗️ Architecture & Crates

GhostPath VPN is organized as a modular Cargo workspace:

| Crate | Purpose |
| :--- | :--- |
| **`ghostpath-core`** | Configuration manager (0600 permissions), state machine, event bus, and routing rules engine. |
| **`ghostpath-proto`** | V2Ray-family URI parsers (`vless://`, `vmess://`, `trojan://`, `ss://`), AmneziaVPN (`vpn://` Qt qCompress), AmneziaWG / WireGuard `.conf` parsers, subscriptions, sing-box/Xray supervisor. |
| **`ghostpath-tun`** | Linux `/dev/net/tun` asynchronous packet I/O via Tokio `AsyncFd`. |
| **`ghostpath-net`** | Policy routing manager (`ip rule`/`ip route`), `UndoJournal`, `DnsManager`, `DnsProxy`, `GeoSpoofManager`, and `KillSwitch`. |
| **`ghostpath-daemon`** | Privileged system daemon (`ghostpathd`), `SO_PEERCRED` Unix socket IPC (`/run/ghostpath/ghostpathd.sock`). |
| **`ghostpath-cli`** | User-friendly CLI client (`ghostpath connect`, `disconnect`, `status`, `doctor`, etc.). |
| **`ghostpath-gui`** | Native Linux desktop application built with Tauri 2 + WebKitGTK and React 19. |

### Desktop Application Details
See [docs/DESKTOP_APP.md](docs/DESKTOP_APP.md) for full architecture, IPC specifications, desktop integration, and troubleshooting.
- **Offline & Zero-Network Webview:** No local HTTP server, no open web ports. Uses Tauri IPC commands and events over `/run/ghostpath/ghostpathd.sock`.
- **System Tray & Single Instance:** Quick connect/disconnect toggle from tray, close-to-hide, single-instance window focusing.
- **VPN Link Associations:** One-click import for `vless://`, `vmess://`, `trojan://`, and `ss://` links.
- **Strict CSP:** Bundled local typography (Vazirmatn Persian, Plus Jakarta Sans, DM Sans, JetBrains Mono) with `connect-src 'none'`.

### Linux Identifiers
- **TUN Interface:** `ghost0` (Address `10.8.0.2/24`, Peer `10.8.0.1`, MTU `1420`)
- **Policy Fwmark:** `0x4750` (`18256` decimal)
- **Routing Table:** `5180`
- **Rule Priorities:** `5180` (bypass fwmark), `5181` (capture non-fwmark)
- **nftables Table:** `inet ghostpath`
- **IPC Socket:** `/run/ghostpath/ghostpathd.sock`

---

## 🚀 Installation & Packaging

### Pre-built Packages
Download the latest `.deb` and `.AppImage` installers (with `SHA256SUMS.txt`) from the [GitHub Releases](../../releases) page.

- **Debian / Ubuntu (.deb):**
  ```bash
  sudo apt install ./ghostpath-vpn_0.1.0_amd64.deb
  ```
  > **Note:** `amneziawg-go` (AmneziaWG/WireGuard) is bundled in the `.deb`. For VLESS, VMess, Trojan, and Shadowsocks profiles, install `sing-box` into `/usr/bin/sing-box` (or `/usr/local/bin/sing-box`).
- **Fedora / RHEL (.rpm):**
  ```bash
  sudo rpm -ivh ghostpath-vpn-0.1.0-1.x86_64.rpm
  ```
- **Arch Linux / AUR:**
  ```bash
  cd packaging/arch && makepkg -si
  ```
- **AppImage:**
  ```bash
  chmod +x GhostPath-VPN-x86_64.AppImage
  ./GhostPath-VPN-x86_64.AppImage
  ```

---

## 💻 CLI Usage

The `ghostpath` CLI talks directly to the privileged daemon via authenticated Unix IPC.

```bash
# Connect to default or active VPN server profile
ghostpath connect

# Connect to a specific profile
ghostpath connect node-frankfurt-01

# Check real-time connection status, speed, and active interface
ghostpath status

# Output JSON for scripts and status bars (Waybar, Polybar, etc.)
ghostpath status --json

# Import a VLESS Reality, VMess, Trojan, or subscription URL
ghostpath import "vless://uuid@server.com:443?security=reality&pbk=...#MyNode" --name "Frankfurt-01"

# Import an AmneziaVPN key (vpn://) or AmneziaWG / WireGuard config file
ghostpath import "vpn://...#MyAmneziaServer"
ghostpath import /path/to/amnezia-warp.conf

# List all stored profiles with protocol badges (VLESS, AmneziaWG, WireGuard, etc.)
ghostpath profiles

# Run comprehensive system diagnostics
ghostpath doctor

# View recent system daemon logs
ghostpath logs -n 50

# Cleanly disconnect and restore routing/DNS
ghostpath disconnect
```

---

## 🛠️ System Doctor (`ghostpath doctor`)

Diagnose your Linux network environment with automated recommendations:

```
$ ghostpath doctor
Running GhostPath System Doctor...
  [PASS] Kernel TUN Device (/dev/net/tun)
  [PASS] Daemon Capabilities / Privileges
  [PASS] Reverse Path Filtering (rp_filter loose/compatible)
  [PASS] IPv4 Packet Forwarding (net.ipv4.ip_forward)
  [PASS] systemd-resolved Integration (~. link routing)
  [PASS] Docker Bridge Isolation (docker0)
  [PASS] IPv6 Leak Protection Posture
  [PASS] Concurrent VPN Interfaces (coexistence active)
  [PASS] AmneziaWG Engine (amneziawg-go userspace binary ready)
```

---

## 🔨 Building from Source

### Prerequisites
- Linux kernel >= 5.10
- Rust 1.75+ (`rustup default stable`)
- Build essentials: `gcc`, `pkg-config`, `libssl-dev`
- GUI dependencies: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libappindicator3-dev`, `librsvg2-dev`
- Frontend dependencies: Node.js 20+ (`npm`)
- System tools: `iproute2`, `iptables` / `nftables`

```bash
# Clone the repository
git clone https://github.com/cybermaz/ghost_path.git
cd ghost_path

# Build offline UI assets
cd UI && npm install --legacy-peer-deps && npm run build && cd ..

# Build all workspace crates in release mode
cargo build --release --workspace

# Build all distribution packages (.deb, AppImage, RPM spec, PKGBUILD)
./scripts/build_all_packages.sh

# Run unit and integration tests across workspace
cargo test --workspace
```

---

## 🔒 Security & Privacy

- **Zero Telemetry:** GhostPath collects zero telemetry, metrics, or personal analytics.
- **Sensitive Credential Redaction:** Passwords, UUIDs, Reality keys, and subscription URLs are redacted from logs and IPC node listings.
- **Privilege Separation:** CLI and GUI run unprivileged; communication with `ghostpathd` over `/run/ghostpath/ghostpathd.sock` (`0660 root:ghostpath`) is authenticated kernel-side using `SO_PEERCRED` and rate-limited.
- **Memory Safety:** `#![forbid(unsafe_code)]` is enforced across all library, CLI, and GUI crates, except minimal documented syscalls in `ghostpath-tun` (`/dev/net/tun` ioctls) and `ghostpathd` process hardening (`PR_SET_DUMPABLE`, `RLIMIT_CORE`, `umask`).
- **Security Docs & Limitations:** See [SECURITY.md](SECURITY.md), [docs/SECURITY_REVIEW/THREAT_MODEL.md](docs/SECURITY_REVIEW/THREAT_MODEL.md), and [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md). GhostPath is a censorship-circumvention tunnel, not an anonymity network: it does not protect against browser fingerprinting, global passive ISP traffic correlation, or compromise of the remote VPN server.

---

## 📄 License

Licensed under either of:
- Apache License, Version 2.0 (`Apache-2.0`)
- MIT license (`MIT`)

at your option.
