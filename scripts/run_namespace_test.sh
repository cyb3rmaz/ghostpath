#!/usr/bin/env bash
set -euo pipefail

export RUSTUP_HOME=/home/cybermaz/.rustup
export CARGO_HOME=/home/cybermaz/.cargo
export PATH="/home/cybermaz/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

cd /mnt/c/Users/cybermaz/Projects/ghost_path

echo "=== Running integration test inside isolated network namespace ==="
unshare -n env \
    RUSTUP_HOME=/home/cybermaz/.rustup \
    CARGO_HOME=/home/cybermaz/.cargo \
    PATH="/home/cybermaz/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
    cargo test -p ghostpath-net --test namespace_routing_test -- --nocapture
