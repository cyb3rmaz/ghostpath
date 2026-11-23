#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
BUILD_DIR="${ROOT_DIR}/target/appimage"
APP_DIR="${BUILD_DIR}/GhostPath.AppDir"
OUTPUT_NAME="GhostPath-VPN-x86_64.AppImage"

echo "=== Building GhostPath VPN AppImage ==="

mkdir -p "${DIST_DIR}"
cd "${ROOT_DIR}"

# Build all workspace release binaries
cargo build --release --workspace

rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/usr/bin"
mkdir -p "${APP_DIR}/usr/share/applications"
mkdir -p "${APP_DIR}/usr/share/metainfo"

# Copy binaries
cp target/release/ghostpath "${APP_DIR}/usr/bin/"
cp target/release/ghostpathd "${APP_DIR}/usr/bin/"
cp target/release/ghostpath-gui "${APP_DIR}/usr/bin/"

# Copy desktop and AppStream metadata
cp packaging/ghostpath.desktop "${APP_DIR}/"
cp packaging/ghostpath.desktop "${APP_DIR}/usr/share/applications/"
cp packaging/ghostpath.metainfo.xml "${APP_DIR}/usr/share/metainfo/"
cp packaging/appimage/AppRun "${APP_DIR}/"
chmod +x "${APP_DIR}/AppRun"

# Install icons in all resolutions
for size in 16 24 32 48 64 128 256 512; do
    mkdir -p "${APP_DIR}/usr/share/icons/hicolor/${size}x${size}/apps"
    cp "packaging/icons/${size}x${size}.png" "${APP_DIR}/usr/share/icons/hicolor/${size}x${size}/apps/ghostpath.png"
done

mkdir -p "${APP_DIR}/usr/share/icons/hicolor/scalable/apps"
cp packaging/ghostpath.svg "${APP_DIR}/usr/share/icons/hicolor/scalable/apps/ghostpath.svg"
cp packaging/ghostpath.svg "${APP_DIR}/ghostpath.svg"
cp packaging/icons/256x256.png "${APP_DIR}/ghostpath.png"

echo "AppDir populated successfully at ${APP_DIR}."

# Check if appimagetool exists
if command -v appimagetool >/dev/null 2>&1; then
    echo "Running appimagetool..."
    ARCH=x86_64 appimagetool --no-appstream "${APP_DIR}" "${DIST_DIR}/${OUTPUT_NAME}" || ARCH=x86_64 appimagetool "${APP_DIR}" "${DIST_DIR}/${OUTPUT_NAME}"
    echo "Generated: ${DIST_DIR}/${OUTPUT_NAME}"
fi

echo "Creating standalone AppDir bundle tarball..."
tar -czf "${DIST_DIR}/GhostPath-VPN-x86_64.AppDir.tar.gz" -C "${BUILD_DIR}" GhostPath.AppDir
echo "Generated AppDir archive: ${DIST_DIR}/GhostPath-VPN-x86_64.AppDir.tar.gz"
