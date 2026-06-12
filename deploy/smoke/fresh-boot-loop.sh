#!/usr/bin/env bash
# Fresh-volume boot-race acceptance loop
# (fix-deploy-smoke-gateway-exit78-fresh-boot-race).
#
# Reproduces + verifies the gateway's first-fresh-boot reliability: runs
# N cycles of `down -v` (wipe the argo-data + pg-data volumes) → `up -d`
# and asserts the gateway reaches a serving /health on EVERY cycle with
# ZERO `exit 78` (EX_CONFIG, the startup-check fail-fast).
#
# This is the DEC-029 evidence harness for the acceptance criterion:
#   "10 consecutive down -v → up -d cycles boot the gateway Healthy with
#    zero exit 78."
# It is NOT a blind `restart: on-failure` — it does not retry the boot;
# it COUNTS the raw first-boot outcome of each fresh cycle. The fix lives
# in the gateway's startup-check (bounded in-process store-readiness
# retry); this loop proves it.
#
# Usage:
#   bash deploy/smoke/fresh-boot-loop.sh            # 10 cycles (default)
#   bash deploy/smoke/fresh-boot-loop.sh 20         # 20 cycles
#   SMOKE_GATEWAY_PORT=8081 bash deploy/smoke/fresh-boot-loop.sh
#
# Builds images once up front (reusing the canonical smoke stack), then
# loops boot/teardown. Tears down on every exit path. Prints a PASS/FAIL
# tally and exits non-zero if any cycle exit-78'd or failed to serve.

set -euo pipefail

CYCLES="${1:-10}"
if ! [[ "${CYCLES}" =~ ^[0-9]+$ ]] || (( CYCLES < 1 )); then
  echo "FAIL: cycle count must be a positive integer (got '${CYCLES}')" >&2
  exit 1
fi

SMOKE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SMOKE_DIR}/../.." && pwd)"
COMPOSE_FILE="${REPO_ROOT}/deploy/docker-compose.smoke.yml"
ENV_FILE="${SMOKE_DIR}/.env.smoke"

# --- preflight ---------------------------------------------------------------

if ! command -v docker >/dev/null 2>&1; then
  echo "FAIL: docker not found in PATH" >&2
  exit 1
fi
if docker compose version >/dev/null 2>&1; then
  COMPOSE=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
  COMPOSE=(docker-compose)
else
  echo "FAIL: neither 'docker compose' nor 'docker-compose' is available" >&2
  exit 1
fi
for f in "${COMPOSE_FILE}" "${ENV_FILE}"; do
  [[ -f "${f}" ]] || { echo "FAIL: missing ${f}" >&2; exit 1; }
done

export SMOKE_GATEWAY_PORT="${SMOKE_GATEWAY_PORT:-8080}"
export SMOKE_SPA_PORT="${SMOKE_SPA_PORT:-3000}"
GATEWAY_URL="${SMOKE_GATEWAY_URL:-http://localhost:${SMOKE_GATEWAY_PORT}}"

set -o allexport
# shellcheck disable=SC1090
source "${ENV_FILE}"
set +o allexport

compose() { "${COMPOSE[@]}" --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" "$@"; }

teardown() {
  local code=$?
  echo "--- final teardown ---" >&2
  compose down -v --remove-orphans >/dev/null 2>&1 || true
  exit "${code}"
}
trap teardown EXIT INT TERM

# --- build once --------------------------------------------------------------

echo "=== building smoke images once (gateway + web) ==="
compose build

# --- the loop ----------------------------------------------------------------

PASS=0
FAIL=0
FAILED_CYCLES=()

# Gateway service is started WITHOUT web to isolate the gateway boot from
# web build/start noise — the race is in the gateway process itself, and
# we want a clean per-cycle first-boot signal. (Set FRESH_BOOT_FULL_STACK=1
# to bring up the full stack, which is the heavier-contention repro the
# bug was first observed under.)
UP_SERVICES=(postgres gateway)
if [[ "${FRESH_BOOT_FULL_STACK:-0}" == "1" ]]; then
  UP_SERVICES=()
fi

gateway_cid() { compose ps -q gateway 2>/dev/null; }

for (( i=1; i<=CYCLES; i++ )); do
  echo ""
  echo "=== cycle ${i}/${CYCLES} :: down -v (fresh volume) -> up -d ==="
  compose down -v --remove-orphans >/dev/null 2>&1 || true
  compose up -d "${UP_SERVICES[@]}"

  # Poll the host-mapped /health with a budget. A clean boot serves well
  # inside this window; a boot that exit-78'd will have an exited gateway
  # container (detected below) long before the budget elapses.
  ok=0
  exit78=0
  deadline=$(( $(date +%s) + 90 ))
  while (( $(date +%s) < deadline )); do
    if curl --fail --silent --max-time 5 --output /dev/null "${GATEWAY_URL}/health"; then
      ok=1
      break
    fi
    cid="$(gateway_cid)"
    if [[ -n "${cid}" ]]; then
      state="$(docker inspect -f '{{.State.Status}}' "${cid}" 2>/dev/null || echo unknown)"
      ec="$(docker inspect -f '{{.State.ExitCode}}' "${cid}" 2>/dev/null || echo '')"
      if [[ "${state}" == "exited" ]]; then
        if [[ "${ec}" == "78" ]]; then exit78=1; fi
        break
      fi
    fi
    sleep 2
  done

  if (( ok == 1 )); then
    echo "    cycle ${i}: PASS (gateway served /health)"
    PASS=$(( PASS + 1 ))
  else
    if (( exit78 == 1 )); then
      echo "    cycle ${i}: FAIL — gateway exit 78 (startup_check / EX_CONFIG)" >&2
    else
      echo "    cycle ${i}: FAIL — gateway did not serve /health within budget" >&2
    fi
    echo "    --- gateway logs (cycle ${i}) ---" >&2
    compose logs --no-color gateway 2>&1 | tail -40 >&2 || true
    FAIL=$(( FAIL + 1 ))
    FAILED_CYCLES+=("${i}")
  fi
done

echo ""
echo "=== fresh-boot-loop result: ${PASS}/${CYCLES} PASS, ${FAIL} FAIL ==="
if (( FAIL > 0 )); then
  echo "FAILED cycles: ${FAILED_CYCLES[*]}" >&2
  exit 1
fi
echo "ALL ${CYCLES} fresh-volume boots reached serving /health with zero exit 78."
