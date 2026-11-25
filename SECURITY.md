# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability in GhostPath VPN (such as a traffic/DNS leak, privilege escalation, IPC bypass, parser crash, or secret exposure), **please do not open a public GitHub issue**.

Instead, report it privately to the maintainers:

- **Email / Contact:** `security@ghostpath.vpn` (or via GitHub Private Vulnerability Reporting on this repository)
- **What to include:**
  - Affected component (`ghostpathd`, `ghostpath-net`, `ghostpath-proto`, `ghostpath-gui`, packaging)
  - GhostPath version, Linux distribution, and kernel version
  - Minimal reproduction steps or proof-of-concept
  - Impact assessment (e.g., IP/DNS leak, local privilege escalation, crash)

We aim to acknowledge reports within 48 hours and provide a fix or mitigation plan within 7 days for Critical and High severity issues.

## Security Architecture & Known Limitations

- **Privilege Separation:** The desktop UI (`ghostpath-gui`) and CLI (`ghostpath`) run as unprivileged user processes and communicate with the privileged system daemon (`ghostpathd`) over a `0660` root:`ghostpath` Unix domain socket authenticated via `SO_PEERCRED`.
- **Fail-Closed Tunneling:** In production builds, if the TUN device, policy routing, DNS configuration, nftables kill switch, or proxy core fails to initialize, `ghostpathd` rolls back partial state and refuses to report a connected tunnel.
- **What GhostPath Does NOT Protect Against:**
  - Browser fingerprinting, cookies, or application-level telemetry
  - Applications that bypass OS DNS/routing or use built-in location APIs independent of GeoClue2/timezone
  - Global passive ISP-level traffic correlation / timing analysis
  - Compromise of the remote VPN server or local root compromise
