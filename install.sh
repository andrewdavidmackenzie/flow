#!/bin/bash
# Install flow data files (context definitions and flowstdlib)
# Run this after extracting the release archive

set -e

# Determine platform-standard data directory
case "$(uname -s)" in
    Linux*)  FLOW_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/flow" ;;
    Darwin*) FLOW_DIR="$HOME/Library/Application Support/flow" ;;
    *)       FLOW_DIR="$HOME/.flow" ;;
esac

# Fall back to legacy location if it exists
if [ ! -d "$FLOW_DIR" ] && [ -d "$HOME/.flow" ]; then
    FLOW_DIR="$HOME/.flow"
fi

echo "Installing flow data to ${FLOW_DIR}/"

# Install flowrcli context definitions
if [ -d "runner/flowrcli" ]; then
    mkdir -p "${FLOW_DIR}/runner/flowrcli"
    cp -r runner/flowrcli/* "${FLOW_DIR}/runner/flowrcli/"
    echo "  Installed flowrcli context definitions"
fi

# Install flowrgui context definitions
if [ -d "runner/flowrgui" ]; then
    mkdir -p "${FLOW_DIR}/runner/flowrgui"
    cp -r runner/flowrgui/* "${FLOW_DIR}/runner/flowrgui/"
    echo "  Installed flowrgui context definitions"
fi

# Install flowstdlib (compiled WASM library)
# Looks for flowstdlib/ in the current directory (from the separate tarball)
if [ -d "flowstdlib" ]; then
    mkdir -p "${FLOW_DIR}/lib/flowstdlib"
    cp -r flowstdlib/* "${FLOW_DIR}/lib/flowstdlib/"
    echo "  Installed flowstdlib library"
else
    echo "  flowstdlib/ not found — download the flowstdlib tarball from the"
    echo "  release, extract it here, and re-run this script to install it."
fi

echo ""
echo "Done. Flow data installed to ${FLOW_DIR}/"
