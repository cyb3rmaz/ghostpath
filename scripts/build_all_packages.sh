#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"

echo "=========================================================="
echo "   GhostPath VPN - Packaging & Distribution Pipeline      "
echo "=========================================================="

mkdir -p "${DIST_DIR}"

# 1. Build offline UI
echo "==> 1. Building offline frontend assets (Vite)..."
cd "${REPO_ROOT}/UI"
npm run build

# 2. Build release binaries
echo "==> 2. Compiling release binaries (ghostpath, ghostpathd, ghostpath-gui)..."
cd "${REPO_ROOT}"
cargo build --release --workspace

# 3. Build Debian package
echo "==> 3. Building Debian package (.deb)..."
bash "${REPO_ROOT}/packaging/build-deb.sh"

# 4. Build AppImage
echo "==> 4. Building AppImage..."
bash "${REPO_ROOT}/packaging/appimage/build-appimage.sh"

# 5. Stage Arch PKGBUILD and RPM spec
echo "==> 5. Staging Arch Linux PKGBUILD and Fedora RPM spec..."
cp "${REPO_ROOT}/packaging/arch/PKGBUILD" "${DIST_DIR}/PKGBUILD"
cp "${REPO_ROOT}/packaging/rpm/ghostpath-vpn.spec" "${DIST_DIR}/ghostpath-vpn.spec"

echo "=========================================================="
echo "Packaging complete! Dist directory contents:"
echo "=========================================================="
ls -lh "${DIST_DIR}"
