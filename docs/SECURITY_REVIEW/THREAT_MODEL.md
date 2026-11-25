# GhostPath VPN Threat Model

**Version:** 0.1.0 Security Audit Baseline  
**Scope:** Linux desktop VPN suite (`ghostpathd`, `ghostpath-gui`, `ghostpath`, `ghostpath-core`, `ghostpath-proto`, `ghostpath-net`, `ghostpath-tun`, packaging & systemd integration).  
**Primary High-Risk Context:** Users operating under aggressive state-level internet censorship and active DPI surveillance where traffic leaks, silent fail-open, privilege escalation, or local metadata exposure create real-world physical/legal harm.

---

## 1. Security Objectives

1. **Fail-Closed Traffic Confidentiality:** When a tunnel is requested or active with Kill Switch enabled, traffic MUST either traverse the encrypted VPN tunnel or be dropped. Silent fallback to mock/direct/plaintext sockets is unacceptable.
2. **Privilege Isolation:** Only authorized members of the root/wheel/sudo/`ghostpath` access boundary may trigger privileged daemon actions over IPC. Unprivileged users must never gain arbitrary file read/write, route poisoning, DNS hijacking, or command execution via `ghostpathd`.
3. **Secret Protection:** Private WireGuard/AmneziaWG keys, VLESS UUIDs, Trojan/Shadowsocks credentials, Reality public/short IDs, and subscription URLs must remain mode `0600` (or `0700` dir), never hit world-readable temp files (`/tmp`), never appear in command-line `argv` (`ps`), logs, crash dumps, or UI error strings, and be zeroized from memory when dropped.
4. **Metadata & Location Consistency:** When Geo Spoofing is enabled, OS timezone (`/etc/localtime`) and GeoClue2 static coordinates (`/etc/geolocation`) must be applied atomically and restored reliably after clean disconnect, service stop, or crash (`--recover`).
5. **Resilience to Untrusted Inputs:** Subscription feeds, deep links (`vless://`, `vpn://`, etc.), `.conf` files, and RPC payloads from the unprivileged GUI must be treated as hostile.

---

## 2. Protected Assets

| ID | Asset | Storage / Location | Confidentiality / Integrity Impact |
|---|---|---|---|
| **A1** | Tunnel credentials (VLESS UUID, Reality `pbk`/`sid`, Trojan/SS passwords, WG/AWG private & preshared keys) | `/var/lib/ghostpath/config/config.toml` (or `~/.config/ghostpath/config.toml`), runtime outbound configs, daemon heap | Compromise allows server impersonation, session tracking, or profile theft |
| **A2** | Subscription feed URLs & tokens | `config.toml`, daemon memory, logs | Exposure reveals user provider accounts and censorship-circumvention endpoints |
| **A3** | Plaintext user network traffic & DNS queries | Kernel TUN (`ghost0` / `awg-ghost0`), socket buffers, `/etc/resolv.conf`, `systemd-resolved` | Leak exposes visited domains, IPs, metadata, and censorship circumvention attempt |
| **A4** | Real client IP, OS timezone, & physical location | Network interfaces, `/etc/localtime`, `/etc/timezone`, `/etc/geolocation`, logs | Leak de-anonymizes user or creates timezone/GeoClue mismatch |
| **A5** | Kernel routing tables, fwmark rules, & nftables firewall state | Policy routing table `5180`, fwmark `0x4750`, `inet ghostpath` nft table, `/var/lib/ghostpath/undo_journal.json` | Corruption causes host lockout, routing loops, or silent bypass of VPN |
| **A6** | Root daemon execution integrity (`ghostpathd`) | `/usr/bin/ghostpathd`, `/run/ghostpath/ghostpathd.sock`, child cores (`sing-box`, `xray`, `amneziawg-go`) | Compromise yields full local `root` / `CAP_NET_ADMIN` / `CAP_NET_RAW` |

---

## 3. Threat Actors

| ID | Actor | Capabilities & Access |
|---|---|---|
| **T1** | **Unprivileged local user** | Local shell/process execution under a non-`ghostpath` UID; access to `/tmp`, `/proc`, env vars, DBus, desktop XDG handlers. |
| **T2** | **Malicious local application** (same UID as GUI user) | Access to `/run/ghostpath/ghostpathd.sock` (if user is in `ghostpath` group), XDG deep links (`vless://...`), clipboard, local unprivileged files. |
| **T3** | **Malicious or compromised UI / WebView content** | Compromised frontend dependency or crafted imported profile strings rendered in WebKitGTK / desktop notifications. |
| **T4** | **Malicious VPN profile / subscription operator** | Arbitrary JSON, INI, Base64, zlib (`vpn://`), URI fields, HTTP redirects, oversized payloads, internal SSRF URLs (`169.254.169.254`, `127.0.0.1`), spoofed TLS/HTTP endpoints. |
| **T5** | **Malicious or compromised VPN exit node** | Sees post-tunnel egress traffic; attempts client-side routing confusion or DNS/ICMP leakage if client isolation fails. |
| **T6** | **Network attacker / ISP / State Censor** | Passive DPI, active probing, DNS poisoning/blocking, TLS MITM, RST injection, subscription endpoint blocking, intermittent packet drops forcing reconnect storms. |

---

## 4. Trust Boundaries

```text
+-------------------------------------------------------------------+
| Trust Boundary B1: Unprivileged User / Malicious App (T1, T2, T3) |
|   - ghostpath-gui (Tauri / WebKitGTK), ghostpath CLI              |
|   - Local xdg-open / browser clicking vless://, vpn://            |
+-------------------------------+-----------------------------------+
                                | JSON-RPC 2.0 over Unix Socket
                                | (/run/ghostpath/ghostpathd.sock)
                                v
+-------------------------------------------------------------------+
| Trust Boundary B2: Privileged System Daemon (ghostpathd, root)    |
|   - Config persistence (/var/lib/ghostpath)                       |
|   - Recovery journals (/var/lib/ghostpath/*.json)                 |
|   - Untrusted Profile / Subscription Parsers                      |
+-------+-------------------------+-----------------------+---------+
        |                         |                       |
        | B3: HTTP/TLS Fetch      | B4: Child Core Exec   | B5: OS Network & Files
        v                         v                       v
+---------------+       +------------------+     +-----------------------+
| Subscription  |       | sing-box / xray  |     | Kernel netlink /      |
| & Geo Servers |       | / amneziawg-go   |     | ip / nft / resolvectl |
| (Untrusted T4)|       | JSON/UAPI/SOCKS  |     | /etc/{localtime,geo*} |
+---------------+       +------------------+     +-----------------------+
```

---

## 5. STRIDE Analysis per Boundary

### Boundary B1: User/GUI/CLI -> `ghostpathd` Unix Socket (`SO_PEERCRED`)
| Category | Threat | Current Control & Gap |
|---|---|---|
| **Spoofing** | Non-authorized user connects to `/run/ghostpath/ghostpathd.sock`. | `SO_PEERCRED` checks UID=0 or groups (`ghostpath`, `wheel`, `sudo`, **`adm`**). **Gap:** Socket created with `0o666` allowing non-group users to open/exhaust connections before rejection, and `adm` (log-reading group on Debian) is overly permissive. |
| **Tampering** | Malicious GUI payload injects flags into `ip`/`nft`/UAPI or writes arbitrary config fields (`mtu=0`, `dnsPrimary="1.2.3.4;..."`). | Serde types enforce field shapes, **Gap:** `tunStack`, `dnsPrimary`, `excludedInterfaces`, and rule patterns lack strict server-side character/range validation before hitting `ip`, `nft`, or core JSON. |
| **Repudiation** | Malicious client triggers connects/disconnects or GeoSpoof changes without audit trail or protocol checks. | Structured tracing logs RPC calls, **Gap:** Subscription URLs logged directly in `import_subscription`; control characters in node names can forge log entries (CWE-117). |
| **Information Disclosure** | Error messages, `getLogs`, or `getStatus` leak keys/UUIDs/raw links or `/tmp` config paths. | `nodes` returns `ServerNode` including `raw_link` (contains full secrets!) to any IPC client. Core runtime configs written to predictable `/tmp/ghostpath_core_{port}.json`. |
| **Denial of Service** | Unprivileged client holds open thousands of connections or sends multi-GB single line over Unix socket. | **Gap:** No `max_line_bytes` bound on `read_line()`, no max concurrent connection semaphore, no per-client request timeout or rate limiting. |
| **Elevation of Privilege** | Symlink/hardlink race or arbitrary path injection via `excludedInterfaces`, `config_dir`, or deep links triggers root file overwrite or CLI option injection (`-o`). | Root daemon invokes `ip`, `nft`, `timedatectl`, `systemctl` via `Command::new`. Some arguments come from settings/configs without leading-`-` rejection. |

### Boundary B2: Deep Links & Frontend -> Tauri Desktop App (`ghostpath-gui`)
| Category | Threat | Current Control & Gap |
|---|---|---|
| **Spoofing / UI Redress** | Browser page spams `vless://` or `vpn://` to trigger instant import or connect. | `parse_vpn_deep_link` only parses scheme; no auto-connect, **Gap:** `App.tsx` opens modal immediately on `deep_link_imported` without explicit confirmation view of target server/host or flood throttling. |
| **Tampering / XSS** | Crafted `#Name` fragment in profile or daemon error string exploits DOM sink or CSP bypass. | React JSX escapes text nodes; CSP `script-src 'self'`. **Gap:** CSP allows `style-src 'self' 'unsafe-inline'`, `withGlobalTauri: true`, and no explicit Tauri `on_navigation` / `on_new_window` blocklist. |

### Boundary B3: Daemon -> Remote Subscription HTTP Fetch (`T4`, `T6`)
| Category | Threat | Current Control & Gap |
|---|---|---|
| **Spoofing / Downgrade** | Plaintext `http://` subscription URL or MITM downgrade alters node endpoints to attacker exit. | `reqwest` uses `rustls`, **Gap:** `http://` URLs are accepted without opt-in/rejection, risking trivial MITM injection of malicious exit nodes under censorship. |
| **SSRF (Tampering / Info Disclosure)** | Subscription URL or 302 Redirect targets `127.0.0.1`, `[::1]`, `169.254.169.254`, or RFC1918 LAN services. | **Gap:** Zero IP/DNS-resolution validation before connect or across HTTP redirects; unlimited body read into RAM (OOM DoS). |
| **Denial of Service** | Slowloris, redirect loop, or 500 MB subscription / `vpn://` qCompress bomb exhausts daemon memory/CPU. | `vpn://` caps uncompressed size at 2 MB, **Gap:** `reqwest` has no response size cap or strict connect/body timeout, and `ini_parser` has no input size/line limit. |

### Boundary B4: Daemon -> Child Proxy/Tunnel Cores (`sing-box`, `xray`, `amneziawg-go`)
| Category | Threat | Current Control & Gap |
|---|---|---|
| **Information Disclosure** | Core JSON written to world-readable `/tmp/ghostpath_core_{port}.json`. | **Critical Gap:** Any local user `T1` can read `/tmp/ghostpath_core_*.json` to steal UUIDs, passwords, and Reality keys, or pre-create symlink if permissions allow. |
| **Tampering (Injection)** | Untrusted AWG parameters (`i1..i5`, `endpoint`, `allowed_ips`) inject `\n` UAPI commands (e.g., rogue `public_key=` or `replace_peers=true`). | **Gap:** `build_uapi_config` interpolates raw strings with `\n` directly into UAPI key=value stream! |
| **Fail-Open / Downgrade** | Missing `sing-box`/`xray` binary falls back to `spawn_mock_socks_listener`; missing `/dev/net/tun` falls back to `is_mock = true`; unparseable node URI falls back to unencrypted `"socks"`. | **Critical Gap:** Users see `"Connected"` in UI while traffic is dropped or unencrypted. |
| **Elevation of Privilege** | Child core processes inherit root privileges unnecessarily (`CAP_SYS_ADMIN`/root UID) or core dumps expose heap keys. | `sing-box` only needs `CAP_NET_ADMIN` / `CAP_NET_RAW` + `CAP_NET_BIND_SERVICE` (or socket mark), and `PR_SET_DUMPABLE=0` / `RLIMIT_CORE=0` are not set. |

### Boundary B5: Daemon -> Host Kernel Network & OS State (`/etc`, netlink, `nft`, `ip`)
| Category | Threat | Current Control & Gap |
|---|---|---|
| **Traffic Leak (Info Disclosure)** | `KillSwitch::enable()` hardcodes `oifname ghost0`, dropping `awg-ghost0` or failing silently if `nft` errors. Startup route/DNS setup occurs BEFORE KillSwitch arms. | **Critical Gap:** Startup/shutdown/crash windows permit plaintext packets outside tunnel; `RouteManager::recover` not invoked by `--recover`. |
| **Tampering / Host Lockout** | `nft flush` or broad `ip rule del` deletes non-GhostPath firewall/VPN rules (Docker, Mullvad, Tailscale). Symlink race on `/etc/localtime` or `/etc/geolocation` overwrites arbitrary system files. | Table `inet ghostpath` is scoped, prio `5180/5181` scoped, **Gap:** Non-`O_NOFOLLOW` writes and ignored error returns during recovery/apply. |
| **State Corruption (DoS)** | `SIGKILL` during `connect` leaves stale fwmark rules, broken `/etc/resolv.conf`, stale `ghostpath` nft table, or spoofed `/etc/localtime` & `/etc/geolocation`. | `GeoSpoofManager` journaled and wired to `--recover`; **Gap:** `UndoJournal` for routes/DNS/killswitch exists in `ghostpath-net` but is NOT triggered on daemon startup or `--recover`. |
