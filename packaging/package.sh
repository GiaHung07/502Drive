#!/usr/bin/env bash
# ==============================================================================
# 502Drive Multi-Platform Release Packager
# Generates Linux portable tarball bundles with checksums
# ==============================================================================
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_DIR"

VERSION="${GITHUB_REF_NAME:-v0.1.0}"
VERSION="${VERSION#v}"
DIST_DIR="$REPO_DIR/dist"
ARCH="${ARCH:-x86_64}"

echo "=========================================================="
echo "    502Drive Release Packager v$VERSION ($ARCH)           "
echo "=========================================================="

mkdir -p "$DIST_DIR/linux"

TARGET_BIN="target/release/502drive"
if [ -f "target/${ARCH}-unknown-linux-gnu/release/502drive" ]; then
    TARGET_BIN="target/${ARCH}-unknown-linux-gnu/release/502drive"
fi

if [ ! -f "$TARGET_BIN" ]; then
    echo "Error: Release binary not found at $TARGET_BIN."
    echo "Please build with 'cargo build --release --bin 502drive' before packaging."
    exit 1
fi

echo "[1/2] Generating Linux portable bundle..."
TAR_DIR="/tmp/502drive-v${VERSION}-linux-${ARCH}"
rm -rf "$TAR_DIR"
mkdir -p "$TAR_DIR/packaging" "$TAR_DIR/scripts"

cp "$TARGET_BIN" "$TAR_DIR/502drive"
cp config.sample.toml "$TAR_DIR/"
cp README.md README.vi.md "$TAR_DIR/"
cp packaging/install.sh "$TAR_DIR/packaging/"
cp packaging/502drive.desktop "$TAR_DIR/packaging/"
cp packaging/502drive-symbolic.svg "$TAR_DIR/packaging/"
cp packaging/502drive-inactive-symbolic.svg "$TAR_DIR/packaging/"
cp packaging/502drive.svg "$TAR_DIR/packaging/"
cp packaging/502drive.png "$TAR_DIR/packaging/"
cp packaging/502drive.ico "$TAR_DIR/packaging/"
cp -r packaging/icons "$TAR_DIR/packaging/"
cp packaging/gdclone-bot.service "$TAR_DIR/packaging/"
cp packaging/502drive-tray.service "$TAR_DIR/packaging/"
cp scripts/502drive-tray.py "$TAR_DIR/scripts/"

BUNDLE_NAME="502drive-v${VERSION}-linux-${ARCH}.tar.gz"
tar -czf "$DIST_DIR/linux/$BUNDLE_NAME" -C /tmp "502drive-v${VERSION}-linux-${ARCH}"
rm -rf "$TAR_DIR"

echo "[2/2] Generating SHA256 checksum..."
cd "$DIST_DIR/linux"
sha256sum "$BUNDLE_NAME" > "$BUNDLE_NAME.sha256"
cd "$REPO_DIR"

echo ""
echo "=========================================================="
echo "                RELEASE ARTIFACTS GENERATED:              "
echo "=========================================================="
ls -lh "$DIST_DIR/linux/"
echo "=========================================================="
