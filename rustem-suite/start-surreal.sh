#!/usr/bin/env bash
# ===========================================================================
# start-surreal.sh — Inicia SurrealDB en Podman (Fedora Rawhide, estándar)
# ===========================================================================
set -euo pipefail

IMAGE="surrealdb/surrealdb:2.2.0"
CONTAINER="rustem-surrealdb"
PORT="8000"
DATA_DIR="${HOME}/rustem-suite-data/surrealdb"

mkdir -p "${DATA_DIR}"

echo "[rustem] Deteniendo contenedor previo de SurrealDB (si existe)..."
podman rm -f "${CONTAINER}" 2>/dev/null || true

echo "[rustem] Iniciando SurrealDB en Podman (puerto ${PORT}, datos en ${DATA_DIR})..."
podman run -d \
    --name "${CONTAINER}" \
    --publish "${PORT}:8000" \
    --volume "${DATA_DIR}:/database" \
    "${IMAGE}" \
    start --user root --pass root --log info 0.0.0.0:8000

echo "[rustem] Esperando a que SurrealDB esté disponible..."
for i in $(seq 1 30); do
    if podman exec "${CONTAINER}" wget --spider -q http://localhost:8000/health 2>/dev/null; then
        echo "[rustem] SurrealDB listo en http://localhost:${PORT}"
        exit 0
    fi
    sleep 1
done

echo "[rustem] ERROR: SurrealDB no respondió en el tiempo esperado." >&2
exit 1
