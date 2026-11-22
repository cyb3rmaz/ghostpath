# GhostPath VPN Troubleshooting Guide (TROUBLESHOOTING.md)

This guide helps diagnose and resolve common issues encountered when using GhostPath VPN on Linux.

---

## 1. Quick Diagnostic: Running `ghostpath doctor`

The first step in any troubleshooting is running the automated diagnosis tool:
```bash
ghostpath doctor
```
Or for machine-readable JSON:
```bash
ghostpath doctor --json
```

`ghostpath doctor` checks:
1. Kernel TUN device access (`/dev/net/tun`)
2. Process privileges (`CAP_NET_ADMIN`, `CAP_NET_RAW` or root helper daemon)
3. Active DNS management stack (`systemd-resolved` D-Bus vs `NetworkManager` vs `/etc/resolv.conf`)
4. Docker / container bridge conflicts (`docker0`, `br-*`)
5. Reverse path filtering status (`rp_filter`)
6. IPv6 leak protection status
7. Conflicting active VPN routing tables (`wg0`, `tun*`, `tailscale0`)
8. GhostPath stale routing state or uncommitted undo journals

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
  Ensure rule `5180` exists:
  ```text
  5180: from all fwmark 0x4750 lookup main
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
  resolvectl domain ghost0 "~."
  ```

### C. Docker Containers Cannot Access Internet
- **Symptom:** Host has internet through GhostPath, but Docker containers inside `docker0` fail to resolve domains.
- **Cause:** Docker iptables forward chain blocks non-established packets or attempts to route through `ghost0`.
- **Fix:** Add `docker0` to GhostPath's excluded interfaces in `~/.config/ghostpath/config.toml`:
  ```toml
  [routing]
  excluded_interfaces = ["docker0", "br-*"]
  ```

### D. Stale State after Hard Reboot or `SIGKILL`
- **Symptom:** Host cannot connect after a sudden power loss while VPN was active.
- **Fix:** GhostPath's undo journal cleans up automatically on next daemon launch:
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
