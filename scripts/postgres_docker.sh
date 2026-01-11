#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

CONTAINER_NAME=${CONTAINER_NAME:-datalchemy-postgres}
POSTGRES_PORT=${POSTGRES_PORT:-5432}
POSTGRES_USER=${POSTGRES_USER:-datalchemy}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-datalchemy}
POSTGRES_DB=${POSTGRES_DB:-datalchemy_crm}
FIXTURES_DIR=${FIXTURES_DIR:-"${ROOT_DIR}/fixtures/sql/postgres"}

if docker ps -a --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
  if ! docker ps --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
    docker start "${CONTAINER_NAME}" >/dev/null
  fi
else
  docker run -d \
    --name "${CONTAINER_NAME}" \
    -p "${POSTGRES_PORT}:5432" \
    -e POSTGRES_USER="${POSTGRES_USER}" \
    -e POSTGRES_PASSWORD="${POSTGRES_PASSWORD}" \
    -e POSTGRES_DB="${POSTGRES_DB}" \
    postgres:15 >/dev/null
fi

ready=0
for _ in $(seq 1 30); do
  if docker exec "${CONTAINER_NAME}" pg_isready -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done

if [ "${ready}" -ne 1 ]; then
  echo "Postgres nao ficou pronto a tempo." >&2
  exit 1
fi

apply_sql() {
  local file="$1"
  docker exec -i -e PGPASSWORD="${POSTGRES_PASSWORD}" "${CONTAINER_NAME}" \
    psql -v ON_ERROR_STOP=1 -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" < "${file}"
}

shopt -s nullglob
for file in "${FIXTURES_DIR}/tables"/*.sql; do
  apply_sql "${file}"
done

for file in "${FIXTURES_DIR}/data"/*.sql; do
  apply_sql "${file}"
done
shopt -u nullglob

echo "Postgres pronto em localhost:${POSTGRES_PORT}"
echo "DATABASE_URL=postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@localhost:${POSTGRES_PORT}/${POSTGRES_DB}"
