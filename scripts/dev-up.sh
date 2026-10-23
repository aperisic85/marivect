#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ ! -f .env ]; then python3 scripts/init-env.py; fi
docker compose up --build -d
docker compose --profile tools run --rm seed
printf '%s\n' 'Marivect started. See HTTP_PORT in .env (default http://localhost:8080).'
