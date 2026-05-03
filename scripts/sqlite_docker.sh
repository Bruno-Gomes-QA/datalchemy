#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

CONTAINER_NAME=${CONTAINER_NAME:-datalchemy-sqlite}
SQLITE_DB_NAME=${SQLITE_DB_NAME:-datalchemy_locadora.db}
FIXTURES_DIR=${FIXTURES_DIR:-"${ROOT_DIR}/fixtures/sql/sqlite"}

# Volume path inside the container where the database is stored.
DB_VOLUME="${ROOT_DIR}/.sqlite-data"
DB_PATH="${DB_VOLUME}/${SQLITE_DB_NAME}"

mkdir -p "${DB_VOLUME}"

# Use an Alpine-based container with sqlite3 pre-installed.
IMAGE="keinos/sqlite3:latest"

# Pull image if needed
if ! docker image inspect "${IMAGE}" >/dev/null 2>&1; then
  docker pull "${IMAGE}"
fi

# Remove old container if it exists (sqlite containers are ephemeral).
if docker ps -a --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
  docker rm -f "${CONTAINER_NAME}" >/dev/null
fi

# Apply table fixtures
shopt -s nullglob
for file in "${FIXTURES_DIR}/tables"/*.sql; do
  cat "${file}" | docker run --rm -i \
    --user "$(id -u):$(id -g)" \
    -v "${DB_VOLUME}:/data" \
    --entrypoint sh \
    "${IMAGE}" \
    -c "sqlite3 /data/${SQLITE_DB_NAME}"
done

# Apply data fixtures
for file in "${FIXTURES_DIR}/data"/*.sql; do
  cat "${file}" | docker run --rm -i \
    --user "$(id -u):$(id -g)" \
    -v "${DB_VOLUME}:/data" \
    --entrypoint sh \
    "${IMAGE}" \
    -c "sqlite3 /data/${SQLITE_DB_NAME}"
done
shopt -u nullglob

echo "SQLite pronto em ${DB_PATH}"
echo "DATABASE_URL=sqlite://${DB_PATH}"
