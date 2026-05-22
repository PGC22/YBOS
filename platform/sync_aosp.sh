#!/bin/bash
# sync_aosp.sh — AOSP source sync workflow
# WHY: Wrapper for 'repo init' and 'repo sync' to ensure consistent environment.

set -euo pipefail

# Configuration
AOSP_DIR="${1:-$HOME/aosp-ybos}"
MANIFEST_URL="https://github.com/PGC22/YBOS.git"
MANIFEST_BRANCH="feat/y2-build-environment" # Update this to 'main' once merged
MANIFEST_FILE="platform/manifests/ybos-aosp.xml"

echo "==> Preparing AOSP sync in: $AOSP_DIR"

# Ensure repo tool is available
if ! command -v repo >/dev/null 2>&1; then
    echo "ERROR: 'repo' tool not found. Please run platform/build_host/setup_ubuntu.sh first."
    exit 1
fi

# Create AOSP directory
mkdir -p "$AOSP_DIR"
cd "$AOSP_DIR"

# Initialize repo if not already done
if [ ! -d ".repo" ]; then
    echo "==> Initializing repo with manifest: $MANIFEST_FILE"
    # Note: Using -u with the YBOS repo and -m for the specific manifest file
    repo init -u "$MANIFEST_URL" -b "$MANIFEST_BRANCH" -m "$MANIFEST_FILE"
else
    echo "==> repo already initialized."
fi

# Sync source
echo "==> Syncing AOSP source (this will take a long time and use ~150GB+ disk space)..."
repo sync -c -j"$(nproc)"

echo "==> AOSP source sync complete!"
echo "Code implemented with help from AI Agents Claude, Codex, Jules."
