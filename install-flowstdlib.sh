#!/bin/bash
# Install flowstdlib (compiled WASM standard library for flow)
#
# Usage:
#   curl -sL https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/install-flowstdlib.sh | bash
#
# Or with a specific version:
#   curl -sL https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/install-flowstdlib.sh | bash -s -- v1.1.0

set -e

REPO="andrewdavidmackenzie/flow"
VERSION="${1:-latest}"

# Determine platform-standard data directory
case "$(uname -s)" in
    Linux*)  FLOW_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/flow" ;;
    Darwin*) FLOW_DIR="$HOME/Library/Application Support/flow" ;;
    MINGW*|MSYS*|CYGWIN*) FLOW_DIR="$APPDATA/flow" ;;
    *)       FLOW_DIR="$HOME/.flow" ;;
esac

# Fall back to legacy location if it exists
if [ ! -d "$FLOW_DIR" ] && [ -d "$HOME/.flow" ]; then
    FLOW_DIR="$HOME/.flow"
fi

# Resolve version tag
if [ "$VERSION" = "latest" ]; then
    TAG=$(curl -sI "https://github.com/${REPO}/releases/latest" | grep -i "^location:" | sed 's/.*tag\///' | tr -d '\r\n')
    if [ -z "$TAG" ]; then
        echo "Error: could not determine latest release tag"
        exit 1
    fi
else
    TAG="$VERSION"
fi

echo "Installing flowstdlib ${TAG} to ${FLOW_DIR}/lib/flowstdlib/"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

URL="https://github.com/${REPO}/releases/download/${TAG}/flowstdlib-${TAG}.tar.xz"
echo "Downloading ${URL}..."
curl -sL "$URL" -o "$TMPDIR/flowstdlib.tar.xz"

cd "$TMPDIR"
tar xJf flowstdlib.tar.xz

if [ ! -d "flowstdlib" ]; then
    echo "Error: flowstdlib/ not found in tarball"
    exit 1
fi

DEST="${FLOW_DIR}/lib/flowstdlib"
mkdir -p "${DEST}"
cp -r flowstdlib/* "${DEST}/"

echo "Done. flowstdlib installed to ${DEST}/"
