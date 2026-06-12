#!/usr/bin/env bash
# Production / ECS image boot smoke — builds the ROOT `Dockerfile` (the image
# the ECS task def runs, NOT deploy/Dockerfile.gateway) and proves the gateway
# (1) boots in the runtime image and (2) SERVES a policy-dependent route.
#
# Why this exists (the divergence trap this brief closes):
# the gateway reads truth-boundary policies + the pitch corpus from disk at
# runtime; a deploy image has no Cargo.toml so workspace_root() falls back to the
# container cwd. If the image doesn't COPY policies/ + skill-library/, a
# policy-dependent route returns `503 pitch_unavailable` ("load
# .../policies/truth-boundary/skycot-public-pitch.toml: No such file"). This
# smoke fails loudly on that, so the root image can never silently regress back
# to a policy-less gateway (the WS5 #2763 failure class).
#
# The probe is `POST /api/v1/pitch/ask/public` — the ANONYMOUS pre-bearer route
# that loads the corpus + both truth-boundary policies before answering. A
# canonical question ("What's actually shipped?") returns 200 with
# source=canonical and NO LLM call, so the smoke needs no provider credential.
#
# Usage:
#   bash deploy/smoke/production-image-boot-smoke.sh        # build -> boot -> probe -> teardown
#   SMOKE_KEEP=1 bash deploy/smoke/production-image-boot-smoke.sh   # keep container for post-mortem
#   PROD_IMAGE_SMOKE_PORT=8090 bash deploy/smoke/production-image-boot-smoke.sh  # dodge a port clash
#
# Exit 0 = green; non-zero = a real boot/policy regression.

set -euo pipefail

SMOKE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SMOKE_DIR}/../.." && pwd)"
ENV_FILE="${SMOKE_DIR}/.env.smoke"

IMAGE_TAG="${PROD_IMAGE_SMOKE_TAG:-argo:prod-image-smoke}"
CONTAINER="${PROD_IMAGE_SMOKE_CONTAINER:-argo-prod-image-smoke}"
HOST_PORT="${PROD_IMAGE_SMOKE_PORT:-8090}"
CONTAINER_PORT=8080

# --- preflight ---------------------------------------------------------------
if ! command -v docker >/dev/null 2>&1; then
  echo "FAIL: docker not found in PATH" >&2
  exit 1
fi
if ! docker info >/dev/null 2>&1; then
  echo "FAIL: docker daemon not reachable" >&2
  exit 1
fi

# Throwaway SQLCipher key (production mode mandates an encrypted-at-rest store)
# + placeholder provider key (satisfies the startup provider-presence check; the
# canonical-hit probe makes no LLM call). Both sourced from the CI smoke env.
# shellcheck disable=SC1090
DB_KEY="$(grep -E '^SKYCOT_DB_KEY_BASE64=' "${ENV_FILE}" | cut -d= -f2-)"
PLACEHOLDER_KEY="$(grep -E '^ANTHROPIC_API_KEY=' "${ENV_FILE}" | cut -d= -f2-)"

cleanup() {
  if [ "${SMOKE_KEEP:-0}" = "1" ]; then
    echo "SMOKE_KEEP=1 — leaving container '${CONTAINER}' up for post-mortem (docker logs ${CONTAINER})." >&2
    return
  fi
  docker rm -f "${CONTAINER}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# --- build -------------------------------------------------------------------
echo "==> building the ROOT Dockerfile (production/ECS image) -> ${IMAGE_TAG}"
docker build -f "${REPO_ROOT}/Dockerfile" -t "${IMAGE_TAG}" "${REPO_ROOT}"

# --- boot --------------------------------------------------------------------
echo "==> booting ${IMAGE_TAG} on host port ${HOST_PORT}"
docker rm -f "${CONTAINER}" >/dev/null 2>&1 || true
# The image CMD binds 0.0.0.0:8080 (non-loopback -> production-auth mode). Pass
# the bypass (disables the JWT-secret boot guard) + the throwaway DB key + the
# placeholder provider key so the gateway boots without real secrets. NEVER set
# ARGO_GATEWAY_AUTH_BYPASS in production.
docker run -d --name "${CONTAINER}" \
  -p "${HOST_PORT}:${CONTAINER_PORT}" \
  -e ARGO_GATEWAY_AUTH_BYPASS=1 \
  -e "SKYCOT_DB_KEY_BASE64=${DB_KEY}" \
  -e "ANTHROPIC_API_KEY=${PLACEHOLDER_KEY}" \
  "${IMAGE_TAG}" >/dev/null

# --- wait for liveness -------------------------------------------------------
echo "==> waiting for gateway liveness"
ready=0
for _ in $(seq 1 60); do
  if curl -fsS -o /dev/null "http://127.0.0.1:${HOST_PORT}/health/live" 2>/dev/null; then
    ready=1
    break
  fi
  # Fail fast if the container already died (e.g. libdbus/load error, exit 127).
  if ! docker ps --filter "name=${CONTAINER}" --filter status=running -q | grep -q .; then
    echo "FAIL: container exited before binding. Logs:" >&2
    docker logs "${CONTAINER}" 2>&1 | tail -30 >&2
    exit 1
  fi
  sleep 1
done
if [ "${ready}" != "1" ]; then
  echo "FAIL: gateway never became live within 60s. Logs:" >&2
  docker logs "${CONTAINER}" 2>&1 | tail -30 >&2
  exit 1
fi

# --- probe the policy-dependent route ----------------------------------------
echo "==> probing POST /api/v1/pitch/ask/public (loads corpus + truth-boundary policies)"
resp="$(curl -fsS -w '__STATUS__%{http_code}' \
  -X POST "http://127.0.0.1:${HOST_PORT}/api/v1/pitch/ask/public" \
  -H 'Content-Type: application/json' \
  -d '{"question":"What is actually shipped?"}' 2>&1 || true)"

status="${resp##*__STATUS__}"
body="${resp%__STATUS__*}"

echo "    status=${status}"
echo "    body=${body}" | head -c 400; echo

# A 503 with a policy/corpus load error is the exact failure class this smoke
# guards: the image shipped without the on-disk policies/corpus.
if echo "${body}" | grep -qiE 'pitch_unavailable|pitch corpus/policies|No such file'; then
  echo "FAIL: policy-dependent route returned a corpus/policy load error — the image" >&2
  echo "      is missing policies/ and/or skill-library/ (policy-less gateway regression)." >&2
  exit 1
fi
if [ "${status}" != "200" ]; then
  echo "FAIL: expected 200 from the policy-dependent route, got ${status}." >&2
  exit 1
fi
if ! echo "${body}" | grep -q '"source":"canonical"'; then
  echo "FAIL: 200 but not a canonical truth-bounded answer — corpus did not load as expected." >&2
  exit 1
fi

echo "PASS: production image boots and serves the truth-bounded policy-dependent route (200, source=canonical)."
