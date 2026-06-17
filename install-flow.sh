#!/bin/bash
# Install the complete flow development environment
#
# Usage:
#   curl -sL https://github.com/andrewdavidmackenzie/flow/releases/latest/download/install-flow.sh | bash
#
# Or with a specific version:
#   curl -sL https://github.com/andrewdavidmackenzie/flow/releases/download/v1.2.0/install-flow.sh | bash

set -e

REPO="andrewdavidmackenzie/flow"
VERSION="${1:-latest}"
INSTALL_DIR="${FLOW_INSTALL_DIR:-$HOME/.flow/bin}"

# Detect platform
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Linux*)
        case "$ARCH" in
            x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
            aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
            armv7*)  TARGET="armv7-unknown-linux-gnueabihf" ;;
            *)       echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    Darwin*)
        TARGET="aarch64-apple-darwin"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        TARGET="x86_64-pc-windows-msvc"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

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

echo "Installing flow ${TAG} for ${TARGET}"
echo ""

# Create temp directory
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

# Download and extract binaries
ARCHIVE="flow-${TAG}-${TARGET}.tar.gz"
if [[ "$TARGET" == *"windows"* ]]; then
    ARCHIVE="flow-${TAG}-${TARGET}.zip"
fi
URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE}"
echo "Downloading binaries: ${ARCHIVE}..."
curl -sL "$URL" -o "$TMPDIR/$ARCHIVE"
cd "$TMPDIR"
if [[ "$ARCHIVE" == *.zip ]]; then
    unzip -q "$ARCHIVE"
else
    tar xzf "$ARCHIVE"
fi

# Install binaries
mkdir -p "$INSTALL_DIR"
for bin in flowc flowrcli flowrgui flowrex flowrdb flowedit; do
    if [ -f "$bin" ] || [ -f "$bin.exe" ]; then
        cp "$bin"* "$INSTALL_DIR/"
        echo "  Installed $bin"
    fi
done

# Install context definitions
if [ -f "install.sh" ]; then
    bash install.sh
fi

# Install flowstdlib
echo ""
echo "Downloading flowstdlib..."
STDLIBURL="https://github.com/${REPO}/releases/download/${TAG}/install-flowstdlib.sh"
curl -sL "$STDLIBURL" | bash -s -- "$TAG"

# Check PATH
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo ""
    echo "⚠ Add the install directory to your PATH:"
    SHELL_NAME=$(basename "$SHELL")
    case "$SHELL_NAME" in
        zsh)  echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.zshrc && source ~/.zshrc" ;;
        bash) echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc && source ~/.bashrc" ;;
        fish) echo "  fish_add_path $INSTALL_DIR" ;;
        *)    echo "  export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
    esac
fi

echo ""
echo "✓ Flow ${TAG} installed successfully!"
echo ""
echo "Verify your installation:"
echo "  flowc --version"
echo "  flowrcli --version"
