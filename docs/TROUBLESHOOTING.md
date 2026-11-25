# GhostPath VPN Troubleshooting Guide

## 1. Daemon Not Running (`Socket file does not exist in /run/ghostpath/`)

Check the systemd service status and logs:

```bash
sudo systemctl status ghostpathd
sudo journalctl -u ghostpathd -n 100 --no-pager
```

Start or restart the daemon:

```bash
sudo systemctl start ghostpathd
```

## 2. Permission Denied on Daemon Socket

Your user must be a member of the `ghostpath` (or `sudo`/`wheel`) group to control `ghostpathd`:

```bash
sudo usermod -aG ghostpath "$USER"
```

Log out and log back in (or run `newgrp ghostpath`) for group membership to take effect.

## 3. Leftover Routes, Kill Switch, or Timezone After a Crash

If `ghostpathd` was terminated unexpectedly (`SIGKILL` or power loss), run the built-in recovery command:

```bash
sudo ghostpathd --recover
```

This disarms the `inet ghostpath` nftables kill switch table, flushes routing table `5180` and rules `5180`/`5181`, reverts `systemd-resolved` / `/etc/resolv.conf`, and restores the original `/etc/localtime` and `/etc/geolocation` state.

## 4. GeoClue2 Coordinates Not Updating

Verify `geoclue-2.0` is installed and active:

```bash
systemctl status geoclue
```

Note that applications running inside strict Flatpak/Snap sandboxes or browsers using Wi-Fi/HTML5 geolocation APIs may still expose hardware or network-derived location unless restricted in browser settings.

## 5. Conflicts with Docker or Other VPNs

GhostPath installs coexistence bypass rules for `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `docker0`, `br-*`, `virbr*`, and `tailscale0`. You can customize `excludedInterfaces` in settings if you use custom bridge names.
