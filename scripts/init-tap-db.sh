#!/bin/bash
# Create databases for Tap sync services
set -e
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    CREATE DATABASE tap;
    GRANT ALL PRIVILEGES ON DATABASE tap TO cospan;
    CREATE DATABASE tap_knots;
    GRANT ALL PRIVILEGES ON DATABASE tap_knots TO cospan;
EOSQL
