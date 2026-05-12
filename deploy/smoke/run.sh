#!/usr/bin/env bash
# K5 deployment smoke — orchestrates the K1 compose stack and runs the
# API-level assertion suite in `checks.ts`.
#
# Designed to be safe to invoke from either CI (`.github/workflows/
# deploy-smoke.yml`) or a developer machine. Tears down on every exit
# path so we never leak containers/volumes.
#
# Usage:
#   bash deploy/smoke/run.sh           # full cycle: build → up → check → down
#   SMOKE_KEEP_STACK=1 bash …          # skip teardown for post-mortem
#
# Env (CI sets via .env.smoke; local devs may export manually):
#   ARGO_GATEWAY_BEARER_TOKEN  bearer token (forwarded to checks.ts)
#   SMOKE_GATEWAY_URL          override gateway URL (default localhost:8080)
#   SMOKE_SPA_URL              override SPA URL     (default localhost:3000)

set -euo pipefail

SMOKE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SMOKE_DIR}/../.." && pwd)"
COMPOSE_DIR="${REPO_ROOT}/deploy/docker"
COMPOSE_FILE="${COMPOSE_DIR}/docker-compose.yml"
ENV_FILE="${SMOKE_DIR}/.env.smoke"

# --- preflight ---------------------------------------------------------------

if ! command -v docker >/dev/null 2>&1; then
  echo "FAIL: docker not found in PATH" >&2
  exit 1
fi

# Prefer modern `docker compose` (v2). Fall back to legacy `docker-compose`.
if docker compose version >/dev/null 2>&1; then
  COMPOSE=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
  COMPOSE=(docker-compose)
else
  echo "FAIL: neither 'docker compose' nor 'docker-compose' is available" >&2
  exit 1
fi

if [[ ! -f "${COMPOSE_FILE}" ]]; then
  echo "FAIL: compose file missing at ${COMPOSE_FILE}" >&2
  exit 1
fi
if [[ ! -f "${ENV_FILE}" ]]; then
  echo "FAIL: env file missing at ${ENV_FILE}" >&2
  exit 1
fi

# Port-conflict detection: bail early rather than racing with whatever
# is already bound. Best-effort — ss/lsof availability varies.
for port in 8080 3000; do
  if (command -v ss >/dev/null 2>&1 && ss -ltn 2>/dev/null | grep -q ":${port} ") \
   || (command -v lsof >/dev/null 2>&1 && lsof -iTCP:"${port}" -sTCP:LISTEN >/dev/null 2>&1); then
    echo "FAIL: port ${port} already bound; smoke needs it free" >&2
    exit 1
  fi
done

set -o allexport
# shellcheck disable=SC1090
source "${ENV_FILE}"
set +o allexport

# --- lifecycle ---------------------------------------------------------------

teardown() {
  local code=$?
  if [[ "${SMOKE_KEEP_STACK:-0}" == "1" ]]; then
    echo "SMOKE_KEEP_STACK=1 set — leaving stack up for inspection" >&2
    exit "${code}"
  fi
  echo "--- tearing down compose stack ---" >&2
  "${COMPOSE[@]}" --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" down -v --remove-orphans \
    >/dev/null 2>&1 || true
  exit "${code}"
}
trap teardown EXIT INT TERM

echo "--- building images ---"
"${COMPOSE[@]}" --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" build

echo "--- starting stack ---"
"${COMPOSE[@]}" --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" up -d

wait_for() {
  local label="$1" url="$2" budget="${3:-90}"
  echo "--- polling ${label} (${url}, ${budget}s budget) ---"
  local deadline=$(( $(date +%s) + budget ))
  while (( $(date +%s) < deadline )); do
    if curl --fail --silent --max-time 5 --output /dev/null "${url}"; then
      echo "    ${label} ready"
      return 0
    fi
    sleep 2
  done
  echo "FAIL: ${label} did not return 200 within ${budget}s" >&2
  "${COMPOSE[@]}" --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" logs --no-color || true
  return 1
}

GATEWAY_URL="${SMOKE_GATEWAY_URL:-http://localhost:8080}"
SPA_URL="${SMOKE_SPA_URL:-http://localhost:3000}"

# The argo image's HEALTHCHECK uses `argo gateway probe live` (no curl
# in distroless). Compose may still report unhealthy briefly during
# boot; poll the host-mapped port directly with a 90s budget.
wait_for "gateway /health" "${GATEWAY_URL}/health" 90
wait_for "spa /"          "${SPA_URL}/"          90

# --- checks ------------------------------------------------------------------

echo "--- running deploy/smoke/checks.ts ---"
cd "${SMOKE_DIR}"
if [[ ! -d node_modules ]]; then
  npm install --no-audit --no-fund --silent
fi

SMOKE_GATEWAY_URL="${GATEWAY_URL}" \
SMOKE_SPA_URL="${SPA_URL}" \
ARGO_GATEWAY_BEARER_TOKEN="${ARGO_GATEWAY_BEARER_TOKEN:-}" \
  npx --no-install ts-node --transpile-only checks.ts | tee checks.out

if grep -q '"ok":false' checks.out; then
  echo "FAIL: one or more smoke checks failed (see checks.out)" >&2
  "${COMPOSE[@]}" --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" logs --no-color || true
  exit 1
fi

echo "--- smoke PASS ---"
