#!/bin/bash
# Fetch the latest Tangled lexicons from tangled.org/tangled.org/core
#
# Usage: ./scripts/fetch-tangled-lexicons.sh
#
# Clones or updates the Tangled core repo and copies lexicons into
# packages/lexicons/sh/tangled/ for codegen.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
VENDOR_DIR="$ROOT_DIR/vendor/tangled-core"
LEXICONS_DST="$ROOT_DIR/packages/lexicons/sh/tangled"
REPO_URL="https://tangled.org/tangled.org/core"

echo "Fetching Tangled lexicons from $REPO_URL ..."

if [ -d "$VENDOR_DIR/.git" ]; then
    echo "  Updating existing clone..."
    cd "$VENDOR_DIR"
    git pull --ff-only
else
    echo "  Cloning..."
    mkdir -p "$(dirname "$VENDOR_DIR")"
    git clone "$REPO_URL" "$VENDOR_DIR"
fi

# Copy lexicons to packages/lexicons/sh/tangled/
echo "  Copying lexicons to $LEXICONS_DST ..."
rm -rf "$LEXICONS_DST"
mkdir -p "$LEXICONS_DST"
cp -r "$VENDOR_DIR/lexicons/"* "$LEXICONS_DST/"

# Show what we got
echo ""
echo "Tangled lexicons updated:"
find "$LEXICONS_DST" -name "*.json" | sort | while read f; do
    nsid=$(python3 -c "import json,sys; print(json.load(open('$f')).get('id','?'))" 2>/dev/null || basename "$f" .json)
    echo "  $nsid"
done
echo ""
echo "Run 'cargo run -p cospan-codegen' to regenerate."
