# GhostPath VPN Troubleshooting Guide (TROUBLESHOOTING.md)

This guide helps diagnose and resolve common issues encountered when running GhostPath VPN on Linux distributions.

---

## 1. Quick Diagnostic: Running `ghostpath doctor`

The first step in diagnosing any connection or routing anomaly is running the automated diagnosis tool:
```bash
ghostpath doctor
```
Or for machine-readable JSON:
```bash
ghostpath doctor --json
```

`ghostpath doctor` automatically inspects:
1. Kernel TUN device access (`/dev/net/tun`)
2. Process privileges and capability bounding sets (`CAP_NET_ADMIN`, `CAP_NET_RAW`)
3. Reverse path filtering status (`net.ipv4.conf.*.rp_filter`)
4. IPv4 packet forwarding (`net.ipv4.ip_forward`)
5. Active DNS management stack (`systemd-resolved` D-Bus vs `NetworkManager` vs `/etc/resolv.conf`)
6. Docker / container bridge conflicts (`docker0`, `br-*`)
7. IPv6 leak protection posture and blackhole routes
8. Concurrent VPN routing tables (`wg*`, `tun*`, `tailscale*`)

---

## 2. Common Issues & Solutions

### A. Routing Loop or Freeze on Connection
- **Symptom:** Network halts immediately when connecting; high CPU utilization.
- **Cause:** Outbound proxy socket was not marked with `SO_MARK = 0x4750` or the policy routing rule is missing.
- **Fix:**
  Verify policy rules:
  ```bash
  ip rule show
  ```
  Ensure rule `5180` and `5181` exist:
  ```text
  5180: from all fwmark 0x4750 lookup main
  5181: from all not fwmark 0x4750 lookup 5180
  ```

### B. DNS Queries Leaking to Local ISP
- **Symptom:** Blocked sites are poisoned with local ISP landing pages even though the tunnel is connected.
- **Cause:** `systemd-resolved` is sending queries to physical link resolvers instead of `ghost0`.
- **Fix:**
  Check resolver status:
  ```bash
  resolvectl status ghost0
  ```
  Ensure `ghost0` has routing domain `"~."`:
  ```bash
  sudo resolvectl domain ghost0 "~."
  sudo resolvectl dns ghost0 10.8.0.1 1.1.1.1
  ```

### C. Packet Loss from Strict Reverse Path Filtering (`rp_filter = 1`)
- **Symptom:** TCP handshake connects, but responses or incoming streams randomly stall.
- **Cause:** Kernel reverse path filtering in strict mode (1) drops incoming packets arriving on asymmetric interfaces.
- **Fix:** Set loose mode (2) on all interfaces:
  ```bash
  sudo sysctl -w net.ipv4.conf.all.rp_filter=2
  sudo sysctl -w net.ipv4.conf.default.rp_filter=2
  ```
  To make it permanent, add to `/etc/sysctl.d/99-ghostpath.conf`:
  ```ini
  net.ipv4.conf.all.rp_filter = 2
  net.ipv4.conf.default.rp_filter = 2
  ```

### D. Docker Containers Cannot Access Internet
- **Symptom:** Host has internet through GhostPath, but Docker containers inside `docker0` fail to resolve domains.
- **Cause:** Docker iptables forward chain blocks non-established packets or attempts to route through `ghost0`.
- **Fix:** Add `docker0` to GhostPath's excluded interfaces in `~/.config/ghostpath/config.toml`:
  ```toml
  [routing]
  excluded_interfaces = ["docker0", "br-*"]
  ```

### E. Multi-VPN / WireGuard Coexistence Conflicts
- **Symptom:** WireGuard (`wg0`) connection drops or fails to handshake when GhostPath connects.
- **Cause:** WireGuard's `AllowedIPs = 0.0.0.0/0` tries to overwrite the default routing table.
- **Fix:**
  In WireGuard config, set `Table = off` or specify a dedicated table, or use GhostPath's Chain Mode where WireGuard traffic rides *inside* GhostPath:
  ```ini
  [Interface]
  PrivateKey = ...
  Address = 10.0.0.2/24
  Table = 5190
  ```

### F. IPC Access Denied (`/run/ghostpath/ghostpathd.sock`)
- **Symptom:** CLI or GUI reports `Access denied: UID 1000 is not authorized to control GhostPath VPN`.
- **Cause:** Client UID is neither root nor in the `ghostpath`, `sudo`, or `wheel` group.
- **Fix:**
  Add your current user to the `ghostpath` or `sudo` group:
  ```bash
  sudo usermod -aG ghostpath $USER
  ```
  Then log out and log back in.

### G. Stale State after Hard Reboot or `SIGKILL`
- **Symptom:** Host cannot connect after a sudden power loss while VPN was active.
- **Fix:** GhostPath's undo journal cleans up automatically on daemon launch:
  ```bash
  sudo ghostpathd --recover
  ```
  Or manually flush GhostPath's dedicated table and rules:
  ```bash
  sudo ip rule del priority 5180 2>/dev/null || true
  sudo ip rule del priority 5181 2>/dev/null || true
  sudo ip route flush table 5180
  sudo nft delete table inet ghostpath 2>/dev/null || true
  ```

### H. Viewing System Daemon Logs
- To inspect real-time logs directly from systemd:
  ```bash
  sudo journalctl -u ghostpathd -f
  ```
- Or via GhostPath CLI:
  ```bash
  ghostpath logs -n 100
  ```
