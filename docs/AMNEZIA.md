# AmneziaVPN & AmneziaWG Technical Specification & Design (docs/AMNEZIA.md)

This document contains verified technical findings, protocol specifications, container mappings, runtime architecture evaluation, and integration designs for adding **AmneziaVPN** and **AmneziaWG** support to **GhostPath VPN**.

---

## 1. Executive Summary & Verification Findings

All research and specifications have been verified against official upstream sources:
- `amnezia-vpn/amnezia-client` (Qt/C++ reference desktop and mobile application)
- `amnezia-vpn/amneziawg-go` (Userspace Go implementation of AmneziaWG)
- `amnezia-vpn/amneziawg-linux-kernel-module` (Linux kernel module fork of WireGuard)
- `amnezia-vpn/amneziawg-tools` (Userspace `awg` and `awg-quick` CLI tools)
- Official protocol documentation at `amnezia.org`

### Verification Summary Table

| Category | Upstream Spec | Verified Fact | GhostPath Status |
| :--- | :--- | :--- | :--- |
| **`vpn://` Payload** | Qt `qCompress` (zlib + 4-byte header) + Base64URL | 4-byte big-endian uncompressed length + standard zlib stream | **Fully Confirmed** |
| **AmneziaWG v1.0** | Junk packets + padding + magic headers | `Jc`, `Jmin`, `Jmax`, `S1`, `S2`, `H1`–`H4` | **Fully Confirmed** |
| **AmneziaWG v2.0+** | Extended padding + signature packets | `S3`, `S4`, `I1`–`I5` custom signature templates | **Fully Confirmed** |
| **Runtime Engine** | `amneziawg-go` (userspace) | MIT licensed, standalone Go binary, standard WireGuard UAPI | **Chosen as Primary** |
| **Kernel Module** | `amneziawg` DKMS kernel module | GPLv2, requires out-of-tree module compilation and headers | **Optional Fastpath** |

---

## 2. Configuration Formats & Parsing Specifications

GhostPath supports four distinct import representations for AmneziaVPN configurations:

### 2.1. `vpn://` URI Format
A `vpn://` link encodes the complete multi-container server definition:
1. **URI Prefix:** `vpn://`
2. **Payload:** URL-safe Base64 (`base64url`), optionally stripped of trailing padding `=`.
3. **Decoded Binary Representation:** Matches Qt's `qCompress` format:
   - **Bytes 0–3:** 32-bit unsigned integer (`uint32`) in **Big-Endian** order specifying the exact uncompressed length in bytes.
   - **Bytes 4..EOF:** Standard **zlib** compressed deflate stream.
4. **Safety & Decompression Bounds:**
   - To prevent zip-bomb denial of service attacks, the uncompressed length is strictly bounded (maximum 2 MB, enforced before and during zlib decompression).
5. **Decoded JSON Schema:**
   ```json
   {
     "containers": [
       {
         "container": "amnezia-awg",
         "awg": {
           "port": "39001",
           "transport_proto": "udp",
           "last_config": "{\"config\":\"[Interface]\\nPrivateKey = ...\\n...\"}"
         }
       }
     ],
     "defaultContainer": "amnezia-awg",
     "description": "Amnezia Server 1",
     "dns1": "1.1.1.1",
     "dns2": "1.0.0.1",
     "hostName": "203.0.113.10",
     "userName": "root"
   }
   ```

### 2.2. AmneziaWG `.conf` Configuration File
Standard WireGuard INI file format extended with custom obfuscation keys in `[Interface]` and `[Peer]`:
```ini
[Interface]
Address = 10.8.0.2/24, fd00:5180::2/64
PrivateKey = <Base64 32-byte private key>
DNS = 1.1.1.1, 1.0.0.1
MTU = 1420

# AmneziaWG Obfuscation Parameters (v1 & v2)
Jc = 4
Jmin = 40
Jmax = 70
S1 = 15
S2 = 50
S3 = 10
S4 = 20
H1 = 1234567891
H2 = 2234567892
H3 = 3234567893
H4 = 4234567894
I1 = <b 0xa1b2c3d4>

[Peer]
PublicKey = <Base64 32-byte public key>
PresharedKey = <Optional Base64 32-byte preshared key>
Endpoint = 203.0.113.10:39001
AllowedIPs = 0.0.0.0/0, ::/0
PersistentKeepalive = 25
```

### 2.3. Amnezia `.vpn` Backup Files
`.vpn` files exported by the Amnezia client contain either:
- The raw uncompressed JSON export file (`serversList` structure).
- Or the raw `vpn://` string on the first line.

### 2.4. Plain WireGuard `.conf` Files
Standard WireGuard configs are parsed identically, with all Amnezia obfuscation parameters defaulting to `None` / `0`.

---

## 3. AmneziaWG Protocol Parameters & Versions

AmneziaWG extends WireGuard to defeat modern Deep Packet Inspection (DPI) systems that perform static signature analysis and packet size clustering:

### 3.1. Junk Packet Parameters (`Jc`, `Jmin`, `Jmax`)
- **`Jc` (Junk Count):** Count of random garbage UDP packets sent immediately prior to every handshake attempt (typical range: `4–12`). This scrambles packet count timing.
- **`Jmin` / `Jmax`:** Range of sizes (in bytes) for generated junk packets (`Jmin <= Jmax`). Validated strictly against `0 <= Jmin <= Jmax <= 1500`.

### 3.2. Packet Padding Parameters (`S1`, `S2`, `S3`, `S4`)
Standard WireGuard handshake and transport packets have predictable, static byte lengths:
- Handshake Initiation: 148 bytes
- Handshake Response: 92 bytes
- Cookie Reply: 64 bytes
- Data Packets: Encapsulated payload + 32-byte overhead

AmneziaWG injects random padding of length `S1`..`S4` to alter these signatures:
- **`S1`:** Handshake initiation message padding (bytes).
- **`S2`:** Handshake response message padding (bytes).
- **`S3`:** Cookie message padding (bytes, v2.0+).
- **`S4`:** Transport data message padding (bytes, v2.0+).

### 3.3. Header Magic / Message Type Overrides (`H1`, `H2`, `H3`, `H4`)
Standard WireGuard identifies packet types via a 4-byte header (`0x01` init, `0x02` response, `0x03` cookie, `0x04` data). AmneziaWG replaces these fixed headers with custom 32-bit unsigned integers:
- **`H1`:** Magic type for Handshake Initiation.
- **`H2`:** Magic type for Handshake Response.
- **`H3`:** Magic type for Cookie.
- **`H4`:** Magic type for Transport Data.
- *Validation constraint:* `H1`, `H2`, `H3`, `H4` must all be non-zero and pairwise distinct (`H1 != H2 != H3 != H4`).

### 3.4. Custom Signature Packets (`I1` through `I5`, v2.0+)
- Defines up to 5 custom packet templates sent sequentially before the handshake (`I1 -> I2 -> I3 -> I4 -> I5`).
- Syntax supports byte dumping (e.g. `<b 0x[hex]>`) and random blocks.

---

## 4. Multi-Container Protocol Support Matrix

A single `vpn://` key can include multiple container definitions on the same server. GhostPath handles each container transparently:

| Container Identifier | Protocol | GhostPath Handling Strategy |
| :--- | :--- | :--- |
| **`amnezia-awg`** | AmneziaWG (Obfuscated WireGuard) | **Supported:** Handled directly via `AwgBackend` (`amneziawg-go`). |
| **`amnezia-wireguard`** | Standard WireGuard | **Supported:** Handled via `AwgBackend` with zero obfuscation. |
| **`amnezia-xray`** | Xray (VLESS Reality / Shadowsocks) | **Supported:** Mapped to existing `ManagedCoreOutbound` (`sing-box`/`xray-core`). |
| **`amnezia-shadowsocks`** | Shadowsocks (2022 / AEAD) | **Supported:** Mapped to existing `ManagedCoreOutbound`. |
| **`amnezia-openvpn-cloak`** | OpenVPN over Cloak | **Recognized, Unsupported:** User notified with explicit explanation. |
| **`amnezia-openvpn`** | Plain OpenVPN | **Recognized, Unsupported:** User notified with explicit explanation. |
| **`amnezia-ipsec`** | IPsec / IKEv2 | **Recognized, Unsupported:** User notified with explicit explanation. |

*Behavior on Multi-Container Import:*
When a `vpn://` key contains multiple containers, GhostPath imports each supported container as an independent profile grouped under the server name, marks the profile marked by `defaultContainer` as active/preferred, and presents an honest notification dialog listing supported imported profiles and any skipped unsupported containers.

---

## 5. Linux Runtime Architecture Evaluation

Three runtime options for running AmneziaWG on Linux were rigorously compared:

```
+-----------------------------------------------------------------------------------------+
|                               Runtime Comparison Matrix                                 |
+----------------------+--------------------+--------------------+------------------------+
| Feature              | (A) amneziawg-go   | (B) Kernel Module  | (C) Native Rust        |
+----------------------+--------------------+--------------------+------------------------+
| Upstream Maintenance | Active (Amnezia)   | Active (Amnezia)   | None on crates.io      |
| License              | MIT                | GPLv2 (Linux)      | N/A                    |
| Root/Kernel Burden   | Zero (Userspace)   | DKMS, Headers, Mod | Zero                   |
| Distro Portability   | 100% Linux Distros | Fragile on Updates | High                   |
| Secure Boot Safe     | Yes                | No (Needs signing) | Yes                    |
| Packaging Burden     | Single Go binary   | Kernel dependent   | None                   |
| Line-Rate Throughput | ~800-1500 Mbps     | ~2000-5000 Mbps    | N/A                    |
| GhostPath Decision   | **PRIMARY (Default)** | **OPTIONAL (Accel)** | **REJECTED**            |
+----------------------+--------------------+--------------------+------------------------+
```

### Decision:
1. **Primary Runtime:** Supervised userspace **`amneziawg-go`** daemon process.
   - Bundled directly inside GhostPath distribution packages (`.deb`, `.rpm`, `AppImage`, `PKGBUILD`).
   - Runs with minimal Linux capabilities (`CAP_NET_ADMIN` on `/dev/net/tun`).
   - Configured via standard WireGuard Userspace API (UAPI) over a dedicated local Unix socket (`/run/ghostpath/awg0.sock`).
2. **Optional Acceleration:** If `amneziawg-linux-kernel-module` is detected (`/sys/module/amneziawg`), GhostPath can optionally utilize netlink configuration to interface directly with kernel-space `awg` devices.

---

## 6. Network Integration, Routing & Security Architecture

Unlike proxy-based outbounds (which terminate in local SOCKS5/HTTP listeners), AmneziaWG operates as an **L3 network tunnel**:

```
+----------------------------------------------------------------------------+
|                        GhostPath Architecture with AWG                     |
|                                                                            |
|  +------------------+     IPC / UAPI      +-----------------------------+  |
|  |   ghostpathd     | ------------------> | amneziawg-go (supervised)   |  |
|  |  (Daemon Core)   |                     | Interface: awg-ghost0       |  |
|  +------------------+                     +--------------+--------------+  |
|           |                                              |                 |
|           v                                              v                 |
|  +------------------+                     +-----------------------------+  |
|  | Policy Routing   |                     | Encrypted Obfuscated UDP    |  |
|  | Table 5180       |                     | SO_MARK: 0x4750             |  |
|  | Rule 5180 / 5181 |                     | (Routes via Table Main)     |  |
|  +------------------+                     +-----------------------------+  |
+----------------------------------------------------------------------------+
```

### 6.1. Routing Loop Prevention
- The UDP socket of `amneziawg-go` communicates with the remote server endpoint (e.g. `203.0.113.10:39001`).
- `amneziawg-go` sets `fwmark 0x4750` on its outbound UDP socket (`WG_PROCESS_MARK=0x4750` or `Fwmark = 0x4750` in UAPI).
- GhostPath's kernel policy rule (`priority 5180: from all fwmark 0x4750 lookup main`) routes all obfuscated UDP traffic directly over the physical internet connection without entering the tunnel table.
- System traffic is directed into table `5180` with default route `default dev awg-ghost0` (or split defaults `0.0.0.0/1`, `128.0.0.0/1`).

### 6.2. DNS Integration
- The tunnel's DNS servers (`DNS = 1.1.1.1, 1.0.0.1`) are bound to `awg-ghost0` using `systemd-resolved` D-Bus calls (`SetLinkDNS`, `SetLinkDomains` with `"~."`).
- All port 53/853 traffic is captured, preventing DNS leaks to ISP servers.

### 6.3. Secret Key Protection
- Private keys and Preshared keys are never written to command-line parameters or system logs.
- Memory holding keys is zeroized upon drop using `zeroize`.
- Persistent config files in `~/.config/ghostpath/profiles/` are stored strictly with permissions `0600`.
- Keys are masked in the GUI by default (`••••••••`).

### 6.4. Lifecycle, Diagnostics & Health Monitoring
- Health state transitions (`Connecting` -> `Connected` -> `Degraded` -> `Failed`).
- Monitored via UAPI `last_handshake_time_sec`. If `now - last_handshake > 180s`, connection transitions to `Degraded` and triggers exponential backoff re-handshake.
- Traffic counters (`rx_bytes`, `tx_bytes`) are read directly from the interface / UAPI and streamed to the UI in real-time.
- On shutdown, `AwgBackend` tears down interface `awg-ghost0`, terminates `amneziawg-go`, and executes the undo journal.
