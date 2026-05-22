#!/bin/bash
# apply_overlay.sh — Apply YBOS customization overlay to AOSP source tree
# WHY: Copies scaffolds from the repository to the active AOSP build tree.

set -euo pipefail

AOSP_ROOT="${1:-$HOME/aosp-ybos}"
OVERLAY_SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKUP=true

echo "==> Applying YBOS overlay to: $AOSP_ROOT"

if [ ! -d "$AOSP_ROOT" ]; then
    echo "ERROR: AOSP root directory not found at $AOSP_ROOT"
    exit 1
fi

# Function to copy with optional backup
copy_with_backup() {
    local src="$1"
    local dest_rel="$2"
    local dest_full="$AOSP_ROOT/$dest_rel"

    mkdir -p "$(dirname "$dest_full")"

    if [ -f "$dest_full" ] && [ "$BACKUP" = true ]; then
        echo "    Backing up existing: $dest_rel"
        cp "$dest_full" "${dest_full}.bak"
    fi

    echo "    Copying: $dest_rel"
    cp "$src" "$dest_full"
}

# Apply device customizations
echo "==> Copying device files..."
find "$OVERLAY_SRC/device" -type f | while read -r file; do
    rel_path="${file#"$OVERLAY_SRC"/}"
    copy_with_backup "$file" "$rel_path"
done

echo "==> Overlay applied successfully!"
echo "Code implemented with help from AI Agents Claude, Codex, Jules."
