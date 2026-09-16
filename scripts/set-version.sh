#!/usr/bin/env bash
# Bump the 502Drive version across every declaration site in one shot.
# Usage: scripts/set-version.sh 0.2.0
set -euo pipefail

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>   (e.g. $0 0.2.0 — no 'v' prefix)"
    exit 1
fi
[[ "$VERSION" == v* ]] && { echo "Do not include the 'v' prefix"; exit 1; }

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_DIR"

set_cargo_version() {
    sed -i "0,/^version = \".*\"/s//version = \"$VERSION\"/" "$1"
}

set_cargo_version Cargo.toml
set_cargo_version src-tauri/Cargo.toml
sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" src-tauri/tauri.conf.json
sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" package.json
sed -i "s/VERSION=\"\${GITHUB_REF_NAME:-.*}\"/VERSION=\"\${GITHUB_REF_NAME:-v$VERSION}\"/" packaging/package.sh
sed -i 's/\$version = "v[0-9.]*"/\$version = "v'"$VERSION"'"/' package-release.ps1

echo "Version set to $VERSION in:"
grep -H '^version' Cargo.toml src-tauri/Cargo.toml
grep -H '"version"' src-tauri/tauri.conf.json package.json
grep -H 'GITHUB_REF_NAME:-' packaging/package.sh
grep -H 'version = "v' package-release.ps1
