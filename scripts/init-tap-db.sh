#!/bin/bash
# Create the tap database for the Tap sync service
set -e
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    CREATE DATABASE tap;
    GRANT ALL PRIVILEGES ON DATABASE tap TO cospan;
EOSQL
