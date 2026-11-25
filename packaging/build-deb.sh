#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"
DEB_NAME="ghostpath-vpn_0.1.0_amd64.deb"
STAGING="/tmp/ghostpath-deb-staging"

echo "==> Preparing Debian package staging directory at ${STAGING}..."
mkdir -p "${DIST_DIR}"
rm -rf "${STAGING}"
mkdir -p "${STAGING}/DEBIAN"
mkdir -p "${STAGING}/usr/bin"
mkdir -p "${STAGING}/lib/systemd/system"
mkdir -p "${STAGING}/usr/share/applications"
mkdir -p "${STAGING}/usr/share/metainfo"

echo "==> Copying binaries..."
cp "${REPO_ROOT}/target/release/ghostpath" "${STAGING}/usr/bin/"
cp "${REPO_ROOT}/target/release/ghostpathd" "${STAGING}/usr/bin/"
cp "${REPO_ROOT}/target/release/ghostpath-gui" "${STAGING}/usr/bin/"
if [ -f "${REPO_ROOT}/packaging/bin/amneziawg-go" ]; then
    cp "${REPO_ROOT}/packaging/bin/amneziawg-go" "${STAGING}/usr/bin/"
elif command -v amneziawg-go >/dev/null 2>&1; then
    cp "$(command -v amneziawg-go)" "${STAGING}/usr/bin/"
fi
chmod 0755 "${STAGING}/usr/bin/"*

echo "==> Copying systemd service..."
cp "${REPO_ROOT}/systemd/ghostpathd.service" "${STAGING}/lib/systemd/system/"
chmod 0644 "${STAGING}/lib/systemd/system/ghostpathd.service"

echo "==> Copying desktop launcher and AppStream metadata..."
cp "${REPO_ROOT}/packaging/ghostpath.desktop" "${STAGING}/usr/share/applications/"
cp "${REPO_ROOT}/packaging/ghostpath.metainfo.xml" "${STAGING}/usr/share/metainfo/com.ghostpath.vpn.metainfo.xml"
chmod 0644 "${STAGING}/usr/share/applications/ghostpath.desktop"
chmod 0644 "${STAGING}/usr/share/metainfo/com.ghostpath.vpn.metainfo.xml"

echo "==> Installing icons in all resolutions..."
for size in 16 24 32 48 64 128 256 512; do
    mkdir -p "${STAGING}/usr/share/icons/hicolor/${size}x${size}/apps"
    cp "${REPO_ROOT}/packaging/icons/${size}x${size}.png" "${STAGING}/usr/share/icons/hicolor/${size}x${size}/apps/ghostpath.png"
done

mkdir -p "${STAGING}/usr/share/icons/hicolor/scalable/apps"
cp "${REPO_ROOT}/packaging/ghostpath.svg" "${STAGING}/usr/share/icons/hicolor/scalable/apps/ghostpath.svg"

echo "==> Writing control file..."
cat << 'EOF' > "${STAGING}/DEBIAN/control"
Package: ghostpath-vpn
Version: 0.1.0
Section: net
Priority: optional
Architecture: amd64
Maintainer: GhostPath Team <dev@ghostpath.vpn>
Depends: libc6, iproute2, iptables | nftables, systemd, libwebkit2gtk-4.1-0 | libwebkit2gtk-4.0-37, libappindicator3-1, geoclue-2.0
Recommends: systemd-resolved
Homepage: https://github.com/cybermaz/ghost_path
Description: GhostPath VPN - Native Linux VPN client with reliable TUN mode
 GhostPath VPN provides censorship-resistant tunneling with reliable TUN mode
 on Linux, supporting AmneziaVPN (vpn://, .conf), AmneziaWG obfuscation,
 VLESS/Reality, VMess, Trojan, Shadowsocks, and WireGuard protocols
 without routing loops, DNS leaks, or conflicts with other VPN clients.
EOF
chmod 0644 "${STAGING}/DEBIAN/control"

echo "==> Writing postinst and prerm scripts..."
cat << 'EOF' > "${STAGING}/DEBIAN/postinst"
#!/bin/sh
set -e

# Create ghostpath system group if it does not exist
if ! getent group ghostpath >/dev/null; then
    groupadd --system ghostpath
fi

# Add invoking sudo user to ghostpath group if available
if [ -n "$SUDO_USER" ] && [ "$SUDO_USER" != "root" ]; then
    usermod -aG ghostpath "$SUDO_USER" >/dev/null 2>&1 || true
fi

# Reload systemd and enable/restart ghostpathd.service
if [ -d /run/systemd/system ]; then
    systemctl --system daemon-reload >/dev/null || true
    systemctl enable ghostpathd.service >/dev/null || true
    systemctl restart ghostpathd.service >/dev/null || true
fi

# Update desktop and icon caches
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q /usr/share/icons/hicolor || true
fi

exit 0
EOF
chmod 0755 "${STAGING}/DEBIAN/postinst"

cat << 'EOF' > "${STAGING}/DEBIAN/prerm"
#!/bin/sh
set -e

if [ -d /run/systemd/system ]; then
    systemctl stop ghostpathd.service >/dev/null || true
    systemctl disable ghostpathd.service >/dev/null || true
fi

# Run recovery pass to guarantee cleanup of routing rules, DNS, and TUN
if [ -x /usr/bin/ghostpathd ]; then
    /usr/bin/ghostpathd --recover >/dev/null 2>&1 || true
fi

exit 0
EOF
chmod 0755 "${STAGING}/DEBIAN/prerm"

echo "==> Building Debian package with dpkg-deb..."
dpkg-deb --build --root-owner-group "${STAGING}" "${DIST_DIR}/${DEB_NAME}"

echo "==> Debian package generated successfully at: ${DIST_DIR}/${DEB_NAME}"
ls -lh "${DIST_DIR}/${DEB_NAME}"

echo "==> Cleaning up staging..."
rm -rf "${STAGING}"
