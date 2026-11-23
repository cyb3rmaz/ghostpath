# GhostPath VPN - Native Linux Desktop Application (`ghostpath-gui`)

This document details the architecture, IPC flow, security model, desktop integration, packaging, and troubleshooting guidelines for the native Linux desktop application (`ghostpath-gui`).

---

## 1. Architecture Overview

`ghostpath-gui` is a native Linux desktop application built with **Tauri 2** and **WebKitGTK** (`webkit2gtk-4.1`). It replaces the previous web browser and local HTTP server requirement with a lightweight, secure native window.

```mermaid
flowchart TD
    subgraph UI ["Frontend (UI/)"]
        React["React 19 + TypeScript + Tailwind CSS v4"]
        Adapter["Thin Adapter (UI/src/services/api.ts)"]
        Fonts["Bundled Fonts (@fontsource/*)"]
    end

    subgraph Tauri ["Tauri 2 Native Shell (ghostpath-gui)"]
        Commands["IPC Commands (rpc_request, check_daemon_status)"]
        Events["Event Loop (state_changed, traffic_updated)"]
        Tray["System Tray (StatusNotifier / AppIndicator)"]
        SingleInstance["Single Instance Window Activator"]
        DeepLink["MIME Handler (vless://, vmess://, etc.)"]
        Mock["In-Memory Mock Fallback"]
    end

    subgraph Daemon ["GhostPath Daemon (ghostpathd)"]
        Socket["/run/ghostpath/ghostpathd.sock (JSON-RPC 2.0)"]
        Core["Proxy Core Supervisor (sing-box / xray)"]
        TunEngine["TUN Device ghost0 (Table 5180, fwmark 0x4750)"]
        DnsMgr["systemd-resolved D-Bus + DoH Proxy"]
    end

    React --> Adapter
    Adapter -- "invoke('rpc_request')" --> Commands
    Adapter -- "listen('traffic_updated')" <-- Events
    Commands --> Socket
    Events <-- Socket
    Commands -. "offline fallback" .-> Mock
    Tray --> React
    SingleInstance --> React
    DeepLink --> React
```

### Key Architectural Tenets:
1. **Fully Offline Frontend:** The UI webview makes **zero outbound network requests**. All external Google Fonts CDN links were replaced with locally bundled npm `@fontsource` packages.
2. **No Local HTTP Server:** The UI does not listen on or communicate with `127.0.0.1` or any TCP port. All communication occurs over Tauri native IPC and the authenticated Unix domain socket (`/run/ghostpath/ghostpathd.sock`).
3. **Small Binary Footprint:** By leveraging WebKitGTK provided by modern Linux desktop distributions, the distribution binary and package size are minimal (~6.3 MB `.deb`, ~8.8 MB AppImage), compared to 150 MB+ for Electron.
4. **Resilient Daemon State Handling:** If `ghostpathd` is not installed, not running, or inaccessible, the GUI renders a clear `DaemonStatusBanner` with one-click copyable terminal remediation commands rather than crashing or showing a blank screen.

---

## 2. IPC & Event Communication Flow

### Command Invocation
The frontend uses a single unified adapter in `UI/src/services/api.ts`:
```typescript
import { invoke } from '@tauri-apps/api/core';

// Pass JSON-RPC requests directly to the daemon
export const callRpc = async (method: string, params: any = {}): Promise<any> => {
  return await invoke('rpc_request', { method, params });
};
```

### Event Streaming
The Tauri background task in `crates/ghostpath-gui/src/ipc/events.rs` monitors daemon state updates and emits window events directly to the webview:
- `state_changed`: Pushed whenever the tunnel status transitions (`disconnected`, `connecting`, `connected`, `failed`).
- `traffic_updated`: Emitted every second with real-time download/upload speeds and accumulated data.
- `deep_link_imported`: Emitted when the user opens a VPN link from a browser or desktop launcher.
- `tray_toggle_connect`: Emitted when clicking the quick connect/disconnect toggle in the system tray menu.

---

## 3. Security & Permission Model

### 1. Unprivileged Execution (Non-Root)
- The GUI runs strictly as the **standard unprivileged user**, never as `root` or with `sudo`.
- All privileged networking operations (allocating `ghost0`, writing routing rules with priority `5181`, configuring `nftables`, and interacting with `systemd-resolved`) remain encapsulated within `ghostpathd`.

### 2. Capabilities Lockdown
Tauri 2 capabilities are locked down in `crates/ghostpath-gui/capabilities/default.json`:
- **Allowed:** `core:default`, `core:event:default`, `deep-link:default`, `notification:default`.
- **Forbidden:** No arbitrary shell execution (`shell`), no arbitrary filesystem reading/writing (`fs`), no external URL opener (`opener`).

### 3. Strict Content Security Policy (CSP)
Configured in `crates/ghostpath-gui/tauri.conf.json`:
```text
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; img-src 'self' data:; connect-src 'none'
```
- `connect-src 'none'`: Explicitly prevents JavaScript running inside the webview from making any fetch, XMLHttpRequest, or WebSocket connections over the network.
- `font-src 'self'`: Forces fonts to load only from bundled assets.

### 4. Daemon Access Control
- The IPC socket `/run/ghostpath/ghostpathd.sock` is owned by `root:ghostpath` with mode `0660`.
- Only users belonging to the `ghostpath` system group (or root) can read and write to the socket.
- The packaging scripts (`postinst` and `ghostpath-vpn.spec`) automatically create the `ghostpath` system group and add the active user (`$SUDO_USER`) during installation.

---

## 4. Desktop Integration

### System Tray & GNOME Fallback
- **Tray Menu:** Provides quick Connect / Disconnect toggle, connection status indicator, "Open GhostPath" window focus, and "Quit GhostPath".
- **Close-to-Hide:** Clicking the window close button (`[X]`) hides the window to the tray rather than terminating the VPN session.
- **GNOME Without AppIndicator Fallback:** On standard GNOME desktop sessions lacking the `gnome-shell-extension-appindicator` extension, tray icons are not supported by the shell. `ghostpath-gui` detects tray initialization errors, logs a clear warning, and automatically forces the main window to remain accessible without crashing or disappearing.

### Single-Instance Activation
- If GhostPath is already running, launching the application again (or clicking a link) does not spawn a redundant process.
- The secondary instance forwards its launch arguments to the primary instance via `tauri-plugin-single-instance`, which automatically unminimizes and focuses the existing window.

### Deep Link & Protocol Associations
- Registered URI schemes: `vless://`, `vmess://`, `trojan://`, `ss://`, `hysteria2://`, `tuic://`.
- Handled via `packaging/ghostpath.desktop` and `tauri-plugin-deep-link`.
- Clicking a VPN link in a web browser or chat application opens GhostPath and prompts to import the configuration profile.

---

## 5. Linux Packaging & Distribution

All packages install the GUI (`ghostpath-gui`), daemon (`ghostpathd`), CLI (`ghostpath`), systemd service, desktop entry, and icons in all standard resolutions (16 to 512 + SVG):

| Package Format | Output Path | Target Distributions |
| :--- | :--- | :--- |
| **Debian (.deb)** | `dist/ghostpath-vpn_0.1.0_amd64.deb` | Ubuntu 22.04 / 24.04, Debian 12, Linux Mint, Pop!_OS |
| **AppImage** | `dist/GhostPath-VPN-x86_64.AppImage` | Portable on any modern glibc Linux distribution |
| **RPM Spec** | `dist/ghostpath-vpn.spec` | Fedora 39+, RHEL 9+, CentOS Stream, openSUSE |
| **Arch PKGBUILD** | `dist/PKGBUILD` | Arch Linux, Manjaro, EndeavourOS (AUR) |

### Building All Packages
Run the automated packaging script from the repository root:
```bash
./scripts/build_all_packages.sh
```

---

## 6. Flatpak Feasibility & Evaluation

Flatpak was thoroughly evaluated as a potential distribution target:
- **Blockers:** A sandboxed Flatpak runs inside bubblewrap user namespaces. It lacks the Linux capabilities (`CAP_NET_ADMIN`, `CAP_NET_RAW`) needed to create TUN interfaces (`/dev/net/tun`), configure policy routing tables, or manipulate `nftables`.
- **Conclusion:** A standalone Flatpak cannot self-install or operate the VPN tunnel independently. It could only function as a client interface to a host daemon that the user previously installed via root packages. Therefore, native `.deb`, `.rpm`, AppImage, and Arch packages are the officially supported formats.

---

## 7. Troubleshooting

### 1. WebKitGTK DMA-BUF Hardware Acceleration Glitches
On systems with proprietary NVIDIA drivers under Wayland or older Mesa drivers, WebKitGTK may occasionally flicker or render black rectangles.
- **Fix:** Launch with software rendering or disabled DMA-BUF:
  ```bash
  WEBKIT_DISABLE_DMABUF_RENDERER=1 ghostpath-gui
  ```

### 2. Wayland vs X11 Display Protocol
`ghostpath-gui` supports both native Wayland and X11. To force a specific backend:
- Force Wayland:
  ```bash
  GDK_BACKEND=wayland ghostpath-gui
  ```
- Force X11:
  ```bash
  GDK_BACKEND=x11 ghostpath-gui
  ```

### 3. Missing Tray Icon on Ubuntu / GNOME
Ubuntu standard GNOME desktop includes AppIndicator support by default. On vanilla Debian GNOME, Arch GNOME, or Fedora Workstation:
- Install the AppIndicator GNOME extension:
  ```bash
  # Fedora:
  sudo dnf install gnome-shell-extension-appindicator
  # Debian/Ubuntu:
  sudo apt-get install gnome-shell-extension-appindicator
  # Arch:
  sudo pacman -S gnome-shell-extension-appindicator
  ```
- Alternatively, the window will remain accessible normally even without a system tray.

### 4. AppImage Daemon Installation
Because an AppImage cannot write to `/usr/bin` or register systemd services directly:
1. Run the AppImage: `./GhostPath-VPN-x86_64.AppImage`
2. If `ghostpathd` is not yet installed on the host, the app displays the installation command:
   ```bash
   sudo systemctl enable --now ghostpathd.service
   ```
   Or install the native `.deb`/`.rpm` package to set up the system service automatically.
