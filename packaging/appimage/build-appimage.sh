#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BUILD_DIR="${ROOT_DIR}/target/appimage"
APP_DIR="${BUILD_DIR}/GhostPath.AppDir"
OUTPUT_NAME="GhostPath-VPN-x86_64.AppImage"

echo "=== Building GhostPath VPN AppImage ==="

cd "${ROOT_DIR}"
cargo build --release -p ghostpath-cli -p ghostpath-daemon

rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/usr/bin"
mkdir -p "${APP_DIR}/usr/share/applications"
mkdir -p "${APP_DIR}/usr/share/icons/hicolor/256x256/apps"

# Copy binaries
cp target/release/ghostpath "${APP_DIR}/usr/bin/"
cp target/release/ghostpathd "${APP_DIR}/usr/bin/"

# Copy desktop and icon assets
cp packaging/ghostpath.desktop "${APP_DIR}/"
cp packaging/ghostpath.desktop "${APP_DIR}/usr/share/applications/"
cp packaging/appimage/AppRun "${APP_DIR}/"
chmod +x "${APP_DIR}/AppRun"

# Create fallback SVG icon if PNG icon not present
cat << 'EOF' > "${APP_DIR}/ghostpath.svg"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <circle cx="50" cy="50" r="45" fill="#0ea5e9"/>
  <path d="M50 20 C35 20 25 32 25 48 C25 68 35 75 35 75 L42 68 L50 75 L58 68 L65 75 C65 75 75 68 75 48 C75 32 65 20 50 20 Z" fill="#ffffff"/>
  <circle cx="42" cy="42" r="5" fill="#0ea5e9"/>
  <circle cx="58" cy="42" r="5" fill="#0ea5e9"/>
</svg>
EOF
cp "${APP_DIR}/ghostpath.svg" "${APP_DIR}/usr/share/icons/hicolor/256x256/apps/ghostpath.svg"

echo "AppDir populated successfully at ${APP_DIR}."

# Check if appimagetool exists
if command -v appimagetool >/dev/null 2>&1; then
    echo "Running appimagetool..."
    appimagetool "${APP_DIR}" "${BUILD_DIR}/${OUTPUT_NAME}"
    echo "Generated: ${BUILD_DIR}/${OUTPUT_NAME}"
else
    echo "Note: 'appimagetool' not found in PATH. AppDir structure is prepared at ${APP_DIR}."
    echo "To produce the .AppImage, download appimagetool and run: appimagetool ${APP_DIR} ${OUTPUT_NAME}"
fi
