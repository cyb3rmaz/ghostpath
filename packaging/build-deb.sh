#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_DIR="${REPO_ROOT}"
DEB_NAME="ghostpath-vpn_0.1.0_amd64.deb"
STAGING="/tmp/ghostpath-deb-staging"

echo "==> Preparing Debian package staging directory at ${STAGING}..."
rm -rf "${STAGING}"
mkdir -p "${STAGING}/DEBIAN"
mkdir -p "${STAGING}/usr/bin"
mkdir -p "${STAGING}/lib/systemd/system"
mkdir -p "${STAGING}/usr/share/applications"
mkdir -p "${STAGING}/usr/share/ghostpath/ui"

echo "==> Copying binaries..."
cp "${REPO_ROOT}/target/release/ghostpath" "${STAGING}/usr/bin/"
cp "${REPO_ROOT}/target/release/ghostpathd" "${STAGING}/usr/bin/"
cp "${REPO_ROOT}/target/release/ghostpath-gui" "${STAGING}/usr/bin/"
chmod 0755 "${STAGING}/usr/bin/"*

echo "==> Copying systemd service..."
cp "${REPO_ROOT}/systemd/ghostpathd.service" "${STAGING}/lib/systemd/system/"
chmod 0644 "${STAGING}/lib/systemd/system/ghostpathd.service"

echo "==> Copying desktop launcher..."
cp "${REPO_ROOT}/packaging/ghostpath.desktop" "${STAGING}/usr/share/applications/"
chmod 0644 "${STAGING}/usr/share/applications/ghostpath.desktop"

echo "==> Copying UI frontend bundle..."
cp -r "${REPO_ROOT}/UI/dist/"* "${STAGING}/usr/share/ghostpath/ui/"
chmod -R a+rX "${STAGING}/usr/share/ghostpath/ui/"

echo "==> Writing control file..."
cat << 'EOF' > "${STAGING}/DEBIAN/control"
Package: ghostpath-vpn
Version: 0.1.0
Section: net
Priority: optional
Architecture: amd64
Maintainer: GhostPath Team <dev@ghostpath.vpn>
Depends: libc6, iproute2, iptables | nftables, systemd
Recommends: systemd-resolved
Homepage: https://github.com/cybermaz/ghost_path
Description: GhostPath VPN - Native Linux VPN client with reliable TUN mode
 GhostPath VPN provides censorship-resistant tunneling with reliable TUN mode
 on Linux, supporting VLESS/Reality, VMess, Trojan, and Shadowsocks protocols
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

# Reload systemd and enable ghostpathd.service
if [ -d /run/systemd/system ]; then
    systemctl --system daemon-reload >/dev/null || true
    systemctl enable ghostpathd.service >/dev/null || true
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
dpkg-deb --build --root-owner-group "${STAGING}" "${OUTPUT_DIR}/${DEB_NAME}"

echo "==> Debian package generated successfully at: ${OUTPUT_DIR}/${DEB_NAME}"
ls -lh "${OUTPUT_DIR}/${DEB_NAME}"

echo "==> Cleaning up staging..."
rm -rf "${STAGING}"
