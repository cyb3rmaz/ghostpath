# UI Inspection & Integration Plan (UI_INTEGRATION.md)

## 1. Inspection Findings

A comprehensive inspection of the `UI/` folder was conducted prior to Phase 1 completion:

- **Frontend Technology Stack:**
  - **Framework:** React 19.0.1 with React DOM 19.0.1
  - **Language & Tooling:** TypeScript 7.0.2 / 5.x, Vite 8.3.0 (`@vitejs/plugin-react` 6.1.1)
  - **Styling:** Tailwind CSS 4.3.3 (`@tailwindcss/vite` 4.3.3)
  - **Icons:** `lucide-react` 0.546.0
  - **Animation:** `motion` 12.23.24
- **Directory Layout:**
  - `UI/src/App.tsx`: Main application shell with navigation bar, responsive sidebar, toast notifications, and section routing.
  - `UI/src/types/vpn.ts`: TypeScript interfaces for `ServerNode`, `Subscription`, `RoutingRule`, `DiagnosticLog`, `AppSettings`, `ConnectionStatus`, `TrafficPoint`.
  - `UI/src/data/vpnData.ts`: Initial mock nodes, subscriptions, rules, logs, and traffic history.
  - `UI/src/components/`:
    - `TunnelView.tsx`: Connect/Disconnect hero button, real-time throughput graph, session timer, node selector, latency ping tools, routing mode and interface mode toggles.
    - `ThroughputChart.tsx`: Real-time SVG line chart rendering upload and download Mbps.
    - `SubscriptionsView.tsx`: Subscription quota display, auto-sync intervals, refresh and delete actions.
    - `RoutingView.tsx`: Split tunneling rules table with priority filtering by match type (`domain`, `process`, `cidr`, `geosite`, `geoip`).
    - `DiagnosticsView.tsx`: Live journal log viewer with level filtering, search, and DNS/IPv6 leak verification card.
    - `SettingsView.tsx`: Configuration for core engine (`sing-box` vs `xray-core`), systemd user service launch on boot, nftables kill switch, IPv6, MTU, SOCKS5 port, and DNS resolvers.
    - `AddProfileModal.tsx`: Modal for importing subscription URLs or manual links (`vless://`, `hysteria2://`, `trojan://`, etc.).

---

## 2. Branding Review

- **Wordmark & Window Title:**
  - The header and sidebar brand wordmark both display **"GhostPath VPN"** (`UI/src/App.tsx` lines 392 and 499).
  - The HTML document title is configured as GhostPath VPN.
- **Interface Identifiers to Align:**
  - In `UI/src/App.tsx` (line 446) and `SettingsView.tsx` (line 89), references to `tun0` were found in mock strings (e.g. `tun0 bound to 10.8.0.42`).
  - These will be updated to display our designated interface identifier **`ghost0`** when connected to the backend.

---

## 3. Integration Method Decision

Per Section 9.2:
- **Primary Integration:** We package the React app with **Tauri** in `ghostpath-gui`, which produces native Linux desktop executables and packages (`.deb`, `.rpm`, AppImage) utilizing the system's webview.
- **Headless & Browser Mode:** For headless environments, remote servers, or users without WebKitGTK installed, `ghostpath-daemon` / `ghostpath-gui` can serve the compiled `UI/dist` on `127.0.0.1:<port>` with strict CORS, CSP, and a per-session random authentication token.
- **IPC Bridge:** Unprivileged `ghostpath-gui` communicates with privileged `ghostpathd` over the Unix domain socket `/run/ghostpath/ghostpathd.sock` using JSON-RPC messages adhering to `docs/API.md`.

---

## 4. UI Gaps vs Backend Capabilities

The following gaps between the current UI and the complete backend capabilities have been identified:

1. **Chain Mode & Coexistence Configuration:**
   - *Backend capability:* Running other VPNs (e.g. WireGuard `wg0`) through GhostPath, or binding GhostPath to another VPN interface.
   - *Current UI:* Has routing mode (`rule`, `global`, `direct`) and interface mode (`tun`, `system-proxy`), but does not have a dedicated control for "Chain Mode" or "Excluded Interfaces" list (e.g., excluding `wg0`, `docker0`, or custom CIDRs).
   - *Recommendation:* Add an "Excluded Interfaces & CIDRs" section in `SettingsView` and an optional "Chain Mode" badge in `TunnelView`.
2. **GhostPath Doctor Integration:**
   - *Backend capability:* `ghostpath doctor` runs diagnostic checks against kernel TUN support, permissions, systemd-resolved, NetworkManager, Docker iptables rules, IPv6 leak posture, and `rp_filter`.
   - *Current UI:* Has a "Run Leak Check" button in `DiagnosticsView` that checks public egress IP, resolver, and IPv6 suppression, but does not present the full system doctor report.
   - *Recommendation:* Wire "Run Leak Check" to trigger the backend doctor engine and display actionable system findings.
3. **Persian (Farsi) Localization & RTL Support:**
   - *Backend/Specs:* Section 8 of rules requires English and Persian docs and UI strings.
   - *Current UI:* Currently all strings in `UI/` are English.
   - *Recommendation:* Add lightweight i18n support in Phase 9 with an EN/FA language switcher and `dir="rtl"` layout toggle.
