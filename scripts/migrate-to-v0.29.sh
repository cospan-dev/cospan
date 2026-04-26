#!/bin/bash
# Cospan v0.28.x → 0.29.0 migration.
#
# 0.29.0 pins panproto to v0.39.0 which removes Object::Schema and
# stores per-file Merkle schema trees instead (panproto/panproto#49).
# Existing on-disk objects from earlier versions are unreadable by
# the new code, so the panproto-vcs store must be wiped before starting
# the new images. After migration, the next git push (via
# git-remote-cospan from phrom) repopulates the store in the new format.
#
# Postgres / Redis / git-mirror data are NOT touched: those stay
# valid across this upgrade.
#
# Run on the prod box from the cospan checkout:
#   ./scripts/migrate-to-v0.29.sh
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

if [ ! -f .env.production ]; then
    echo ".env.production not found in $(pwd)"
    exit 1
fi

PROJECT_NAME=$(basename "$ROOT_DIR")
NODE_VOLUME="${PROJECT_NAME}_node-data"

echo "Project    : $PROJECT_NAME"
echo "Volume     : $NODE_VOLUME"
echo

# 1. Stop the stack so nothing is reading the volume.
echo "Stopping containers..."
docker compose -f docker-compose.prod.yml --env-file .env.production down

# 2. Drop the panproto-vcs volume (objects + refs + import marks).
#    Keeping it would surface deserialization errors in v0.39.0.
echo "Removing $NODE_VOLUME (old panproto-vcs store)..."
docker volume rm "$NODE_VOLUME" || {
    echo "  volume not found or already removed"
}

# 3. Pull the new images.
echo "Pulling 0.29.0 images..."
COSPAN_VERSION=0.29.0 \
    docker compose -f docker-compose.prod.yml --env-file .env.production pull

# 4. Start the stack on the new images.
echo "Starting containers on 0.29.0..."
COSPAN_VERSION=0.29.0 \
    docker compose -f docker-compose.prod.yml --env-file .env.production up -d

echo
echo "Migration complete."
echo
echo "Next steps:"
echo "  1. Wait for healthchecks: docker compose -f docker-compose.prod.yml ps"
echo "  2. From your local cospan checkout, repopulate the node by"
echo "     pushing through git-remote-cospan:"
echo "         git push panproto main      # uses panproto:// remote"
echo "     The first push parses every file once (~14 min for cospan),"
echo "     subsequent pushes only re-parse blobs that changed thanks"
echo "     to the persistent blob_to_schema cache."
