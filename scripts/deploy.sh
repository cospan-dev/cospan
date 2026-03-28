#!/bin/bash
# Cospan production deployment script
# Usage: ./scripts/deploy.sh
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$ROOT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}Cospan Deployment${NC}"
echo "================================"

# Check prerequisites
if ! command -v docker &>/dev/null; then
    echo -e "${RED}Docker is not installed. Install it first.${NC}"
    exit 1
fi

if ! docker compose version &>/dev/null; then
    echo -e "${RED}Docker Compose is not available.${NC}"
    exit 1
fi

# Check .env.production exists
if [ ! -f .env.production ]; then
    echo -e "${RED}.env.production not found.${NC}"
    echo "Copy .env.production.example to .env.production and fill in values:"
    echo "  cp .env.production.example .env.production"
    exit 1
fi

# Source env to validate
set -a
source .env.production
set +a

if [ -z "$DOMAIN" ]; then
    echo -e "${RED}DOMAIN is not set in .env.production${NC}"
    exit 1
fi

if [ -z "$POSTGRES_PASSWORD" ]; then
    echo -e "${RED}POSTGRES_PASSWORD is not set in .env.production${NC}"
    echo "Generate one with: openssl rand -base64 32"
    exit 1
fi

if [ -z "$NODE_DID" ] || [ "$NODE_DID" = "did:plc:your-did-here" ]; then
    echo -e "${YELLOW}Warning: NODE_DID not configured. Node will use a placeholder DID.${NC}"
fi

echo -e "${GREEN}Domain:${NC} $DOMAIN"
echo -e "${GREEN}Node DID:${NC} $NODE_DID"
echo ""

# Build and deploy
echo -e "${YELLOW}Building images...${NC}"
docker compose -f docker-compose.prod.yml --env-file .env.production build

echo -e "${YELLOW}Starting services...${NC}"
docker compose -f docker-compose.prod.yml --env-file .env.production up -d

echo ""
echo -e "${YELLOW}Waiting for services to be healthy...${NC}"
sleep 10

# Check health
for service in db redis appview node web caddy; do
    status=$(docker inspect --format='{{.State.Status}}' "cospan-${service}" 2>/dev/null || echo "not found")
    if [ "$status" = "running" ]; then
        echo -e "  ${GREEN}✓${NC} cospan-${service}"
    else
        echo -e "  ${RED}✗${NC} cospan-${service} ($status)"
    fi
done

echo ""
echo -e "${GREEN}Deployment complete!${NC}"
echo ""
echo "Your Cospan instance should be available at:"
echo "  Frontend:  https://$DOMAIN"
echo "  Node:      https://node.$DOMAIN"
echo "  AppView:   https://$DOMAIN/xrpc/"
echo ""
echo "Useful commands:"
echo "  docker compose -f docker-compose.prod.yml --env-file .env.production logs -f"
echo "  docker compose -f docker-compose.prod.yml --env-file .env.production ps"
echo "  docker compose -f docker-compose.prod.yml --env-file .env.production down"
