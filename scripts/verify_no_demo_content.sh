#!/usr/bin/env bash
set -euo pipefail

echo "=========================================================="
echo " GhostPath VPN: Automated Zero-Demo Content Scanner"
echo "=========================================================="

FAILED=0

# Ensure UI is built
if [ ! -d "UI/dist" ]; then
    echo "[*] UI/dist directory not found. Building UI frontend..."
    (cd UI && npm run build)
fi

echo "[*] Scanning UI source code (UI/src) for forbidden simulation patterns..."
FORBIDDEN_SRC_PATTERNS=(
    "Math.random"
    "lorem"
    "sample-nordic"
    "mock-node"
    "Baltic Edge Relay"
    "Personal Hetzner"
    "84.17.52.109"
    "sub.nordic-relay.net"
    "vault.mazyar-ops.org"
    "Paste Sample"
    "coming soon"
)

for pattern in "${FORBIDDEN_SRC_PATTERNS[@]}"; do
    if grep -riI "$pattern" UI/src/ >/dev/null 2>&1; then
        echo "  [FAIL] Forbidden pattern found in UI/src: '$pattern'"
        grep -riIn "$pattern" UI/src/ || true
        FAILED=1
    else
        echo "  [PASS] UI/src: No occurrence of '$pattern'"
    fi
done

echo ""
echo "[*] Scanning UI distribution bundle (UI/dist)..."
FORBIDDEN_DIST_PATTERNS=(
    "lorem"
    "sample-nordic"
    "mock-node"
    "Baltic Edge Relay"
    "Personal Hetzner"
    "84.17.52.109"
    "sub.nordic-relay.net"
    "vault.mazyar-ops.org"
    "Paste Sample"
    "coming soon"
)

for pattern in "${FORBIDDEN_DIST_PATTERNS[@]}"; do
    if grep -riI "$pattern" UI/dist/ >/dev/null 2>&1; then
        echo "  [FAIL] Forbidden pattern found in UI/dist: '$pattern'"
        grep -riIn "$pattern" UI/dist/ || true
        FAILED=1
    else
        echo "  [PASS] UI/dist: No occurrence of '$pattern'"
    fi
done

echo ""
echo "[*] Scanning release Rust binaries & libraries for MockDaemon..."
if [ -f "target/release/libghostpath_gui.rlib" ] || compgen -G "target/release/deps/libghostpath_gui-*.rlib" >/dev/null; then
    if strings target/release/libghostpath_gui.rlib target/release/deps/libghostpath_gui-*.rlib 2>/dev/null | grep -i "MockDaemon" >/dev/null 2>&1; then
        echo "  [FAIL] MockDaemon found compiled into release libghostpath_gui"
        FAILED=1
    else
        echo "  [PASS] target/release/libghostpath_gui: MockDaemon is completely absent in release binary"
    fi
else
    echo "  [INFO] target/release/deps not yet built; verifying ghostpath-daemon source..."
fi

if grep -rn "pub mod mock" crates/ghostpath-daemon/src/ >/dev/null 2>&1; then
    echo "  [FAIL] 'pub mod mock' found in ghostpath-daemon!"
    FAILED=1
else
    echo "  [PASS] ghostpath-daemon: contains zero mock modules"
fi

echo ""
if [ $FAILED -ne 0 ]; then
    echo "[-] Verification failed: Forbidden demo, fake, or mock content detected!"
    exit 1
else
    echo "[+] Verification successful: Zero demo, fake, or mock artifacts found in release paths."
fi
