#!/bin/bash
# Install flow binaries and context definitions
# Run this after extracting the release archive

set -e

FLOW_DIR="${HOME}/.flow"

echo "Installing flow context definitions to ${FLOW_DIR}/"

# Install flowrcli context definitions
FLOWRCLI_DIR="${FLOW_DIR}/runner/flowrcli"
if [ -d "runner/flowrcli" ]; then
    mkdir -p "${FLOWRCLI_DIR}"
    cp -r runner/flowrcli/* "${FLOWRCLI_DIR}/"
    echo "  Installed flowrcli context definitions"
fi

# Install flowrgui context definitions
FLOWRGUI_DIR="${FLOW_DIR}/runner/flowrgui"
if [ -d "runner/flowrgui" ]; then
    mkdir -p "${FLOWRGUI_DIR}"
    cp -r runner/flowrgui/* "${FLOWRGUI_DIR}/"
    echo "  Installed flowrgui context definitions"
fi

echo "Done. Context definitions installed to ${FLOW_DIR}/runner/"
echo ""
echo "To also install flowstdlib, download the flowstdlib tarball from the"
echo "release and extract it to ${FLOW_DIR}/lib/"
