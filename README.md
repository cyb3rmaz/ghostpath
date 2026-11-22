# GhostPath VPN 🛡️

[![CI](https://img.shields.io/badge/CI-Passing-brightgreen.svg)](#testing)
[![Linux Only](https://img.shields.io/badge/Platform-Linux%20x86__64%20%7C%20aarch64-blue.svg)](#requirements)
[![Rust Edition](https://img.shields.io/badge/Rust-2021%20(1.75%2B)-orange.svg)](#building-from-source)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-lightgrey.svg)](LICENSE)
[![Farsi Documentation](https://img.shields.io/badge/Docs-%D9%81%D8%A7%D8%B1%D8%B3%DB%8C%20(Persian)-green.svg)](README_FA.md)

**GhostPath VPN** is a high-performance, native Linux VPN client written in Rust, engineered specifically to provide **bulletproof, reliable TUN mode** under severe and hostile internet filtering conditions (such as the national censorship infrastructure in Iran).

Unlike conventional VPN clients that suffer from routing loops, DNS leaks, and systemd-resolved conflicts on Linux, GhostPath VPN employs kernel-level policy routing, socket fwmark exclusion, strict encrypted DNS tunneling, and robust multi-VPN coexistence.

---

## 📖 Translations

- [راهنمای فارسی (Persian / Farsi Documentation)](README_FA.md)

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
* **GhostPath's Solution:** Native integration with `systemd-resolved` over D-Bus/`resolvectl` using routing domain `"~."` on interface `ghost0`, forcing 100% of DNS resolution into the tunnel. Includes an internal encrypted DNS proxy (`DoH` over tunnel) and atomic fallback for `/etc/resolv.conf`.

### 3. Multi-VPN Coexistence & Chaining
* **The Problem:** Running WireGuard, OpenVPN, or Tailscale simultaneously with other clients results in routing table clashes and broken tunnels.
* **GhostPath's Solution:** Dedicated routing table (`5180`) and priority spacing (`5180`/`5181`) leave room for external VPN interfaces. External VPN traffic can ride *inside* GhostPath (WireGuard-over-GhostPath chain mode) with zero collisions.

### 4. Zero-Leak Crash Safety & Atomic Recovery
* **The Problem:** If a VPN daemon crashes or receives `SIGKILL`, leftover routes and altered DNS leave the user with dead internet.
* **GhostPath's Solution:** Atomic `UndoJournal` (`/var/lib/ghostpath/undo_journal.json`). On any exit, signal (`SIGTERM`, `SIGINT`), or crash, `ghostpathd --recover` automatically rolls back all routing rules, routes, and DNS configurations.

### 5. Strict nftables Kill Switch
* Dedicated `inet` table `ghostpath` drops any non-local, non-tunnel traffic if the connection drops unexpectedly, while preserving LAN connectivity and DHCP leases.

---

## 🏗️ Architecture & Crates

GhostPath VPN is organized as a modular Cargo workspace:

| Crate | Purpose |
| :--- | :--- |
| **`ghostpath-core`** | Configuration manager (0600 permissions), state machine, event bus, and routing rules engine. |
| **`ghostpath-proto`** | V2Ray-family URI parsers (`vless://`, `vmess://`, `trojan://`, `ss://`), subscriptions, sing-box/Xray supervisor. |
| **`ghostpath-tun`** | Linux `/dev/net/tun` asynchronous packet I/O via Tokio `AsyncFd`. |
| **`ghostpath-net`** | Netlink policy routing manager, `UndoJournal`, `DnsManager`, `DnsProxy`, and `KillSwitch`. |
| **`ghostpath-daemon`** | Privileged system daemon (`ghostpathd`), `SO_PEERCRED` Unix socket IPC (`/run/ghostpath/ghostpathd.sock`). |
| **`ghostpath-cli`** | User-friendly CLI client (`ghostpath connect`, `disconnect`, `status`, `doctor`, etc.). |
| **`ghostpath-gui`** | Native desktop UI bridge connecting the React 19 frontend to `ghostpathd`. |

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
- **Debian / Ubuntu (.deb):**
  ```bash
  sudo dpkg -i ghostpath-vpn_0.1.0_amd64.deb
  sudo apt-get install -f
  ```
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
```

---

## 🔨 Building from Source

### Prerequisites
- Linux kernel >= 5.10
- Rust 1.75+ (`rustup default stable`)
- Build essentials: `gcc`, `pkg-config`, `libssl-dev`
- System tools: `iproute2`, `iptables` / `nftables`

```bash
# Clone the repository
git clone https://github.com/cybermaz/ghost_path.git
cd ghost_path

# Build all workspace crates in release mode
cargo build --release

# Run unit tests across workspace
cargo test --workspace

# Run network namespace integration tests (requires root or unshare -n)
sudo bash scripts/run_namespace_test.sh
```

---

## 🔒 Security & Privacy

- **Zero Telemetry:** GhostPath collects zero telemetry, metrics, or personal analytics.
- **Sensitive Credential Redaction:** Passwords, UUIDs, Reality keys, and auth headers are strictly redacted from logs.
- **Privilege Separation:** CLI and GUI run unprivileged; communication with `ghostpathd` is authenticated kernel-side using `SO_PEERCRED`.
- **Memory Safety:** `#![forbid(unsafe_code)]` enforced across all crates, except strictly scoped, documented FFI wrappers for `/dev/net/tun` ioctls.

---

## 📄 License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
