#!/usr/bin/env bash
set -euo pipefail

# Manage the test database container.
# Usage:
#   ./scripts/test-db.sh up      # start the test DB
#   ./scripts/test-db.sh down    # stop and remove
#   ./scripts/test-db.sh status  # check if running

COMPOSE_FILE="docker-compose.test.yml"
PROJECT="cospan-test"

case "${1:-up}" in
  up)
    docker compose -f "$COMPOSE_FILE" -p "$PROJECT" up -d --wait
    echo "Test DB ready at postgres://cospan:cospan@localhost:5433/cospan_test"
    ;;
  down)
    docker compose -f "$COMPOSE_FILE" -p "$PROJECT" down -v
    ;;
  status)
    docker compose -f "$COMPOSE_FILE" -p "$PROJECT" ps
    ;;
  *)
    echo "Usage: $0 {up|down|status}" >&2
    exit 1
    ;;
esac
