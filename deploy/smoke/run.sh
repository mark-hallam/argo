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
# Smoke variant of the canonical stack: builds deploy/Dockerfile.gateway +
# deploy/Dockerfile.web with host ports exposed (see deploy/docker-compose.smoke.yml).
COMPOSE_FILE="${REPO_ROOT}/deploy/docker-compose.smoke.yml"
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

# --- monitoring-stack config validation --------------------------------------
# Turns the AUTHORED monitoring config (deploy/{prometheus,alertmanager,grafana})
# into a CHECKED gate (DEC-029: "authored + builds" is not "works"). Three asserts:
#   1. the canonical compose's `--profile monitoring` parses (config -q),
#   2. the Prometheus config + alert rules load (promtool check),
#   3. the rendered Alertmanager config is valid (amtool check-config).
# promtool/amtool run from local PATH if present, else from the prom images
# (no daemon build needed — `docker run` for the tool only). The compose
# config -q gate is mandatory; the prom-tool asserts degrade to WARN only if
# neither a local binary nor a usable image is reachable (air-gapped CI).
validate_monitoring_stack() {
  local deploy_dir="${REPO_ROOT}/deploy"
  local compose_canonical="${deploy_dir}/docker-compose.yml"
  echo "--- validating monitoring stack config ---"

  # config -q needs deploy/.env.deploy to exist (gateway service env_file).
  # Provision a throwaway copy from the example just for the parse, then remove.
  local provisioned_env=0
  if [[ ! -f "${deploy_dir}/.env.deploy" && -f "${deploy_dir}/.env.deploy.example" ]]; then
    cp "${deploy_dir}/.env.deploy.example" "${deploy_dir}/.env.deploy"
    provisioned_env=1
  fi
  local rc=0
  if ! "${COMPOSE[@]}" -f "${compose_canonical}" --profile monitoring config -q; then
    echo "FAIL: 'docker compose --profile monitoring config -q' rejected the stack" >&2
    rc=1
  fi
  (( provisioned_env )) && rm -f "${deploy_dir}/.env.deploy"
  (( rc )) && return 1

  # promtool: validate prometheus.yml + the alert rules it references.
  local prom_img="prom/prometheus:v2.53.0"
  if command -v promtool >/dev/null 2>&1; then
    promtool check rules "${deploy_dir}/alertmanager/rules.yml" || return 1
  elif docker image inspect "${prom_img}" >/dev/null 2>&1 || docker pull -q "${prom_img}" >/dev/null 2>&1; then
    docker run --rm -v "${deploy_dir}/alertmanager/rules.yml:/rules.yml:ro" \
      --entrypoint promtool "${prom_img}" check rules /rules.yml || return 1
  else
    echo "WARN: promtool unavailable (no local binary, image unreachable) — skipping rule check" >&2
  fi

  # amtool: render the alertmanager template with placeholder values (mirrors
  # the compose entrypoint sed) and validate the result.
  local am_img="prom/alertmanager:v0.27.0"
  local rendered; rendered="$(mktemp)"
  sed \
    -e "s|__SMTP_HOST__|email-smtp.example.amazonaws.com|g" \
    -e "s|__SMTP_PORT__|587|g" \
    -e "s|__SMTP_USERNAME__|AKIAEXAMPLE|g" \
    -e "s|__SMTP_PASSWORD__|placeholder|g" \
    -e "s|__SMTP_FROM__|noreply@skycot.com|g" \
    -e "s|__ALERT_EMAIL_TO__|ops@skycot.com|g" \
    "${deploy_dir}/alertmanager/alertmanager.tmpl.yml" > "${rendered}"
  if command -v amtool >/dev/null 2>&1; then
    amtool check-config "${rendered}" || { rm -f "${rendered}"; return 1; }
  elif docker image inspect "${am_img}" >/dev/null 2>&1 || docker pull -q "${am_img}" >/dev/null 2>&1; then
    docker run --rm -v "${rendered}:/am.yml:ro" \
      --entrypoint amtool "${am_img}" check-config /am.yml || { rm -f "${rendered}"; return 1; }
  else
    echo "WARN: amtool unavailable (no local binary, image unreachable) — skipping config check" >&2
  fi
  rm -f "${rendered}"
  echo "--- monitoring stack config OK ---"
}

validate_monitoring_stack

# Host ports are parameterized so a run can dodge a collision with an
# already-bound dev gateway/SPA (CI uses the defaults; a dev box can override,
# e.g. SMOKE_SPA_PORT=3001 when :3000 is taken). These are exported so the
# compose host-port mappings (`${SMOKE_GATEWAY_PORT:-8080}:8080` etc.) resolve.
export SMOKE_GATEWAY_PORT="${SMOKE_GATEWAY_PORT:-8080}"
export SMOKE_SPA_PORT="${SMOKE_SPA_PORT:-3000}"

# Port-conflict detection: bail early rather than racing with whatever
# is already bound. Best-effort — ss/lsof availability varies.
for port in "${SMOKE_GATEWAY_PORT}" "${SMOKE_SPA_PORT}"; do
  if (command -v ss >/dev/null 2>&1 && ss -ltn 2>/dev/null | grep -q ":${port} ") \
   || (command -v lsof >/dev/null 2>&1 && lsof -iTCP:"${port}" -sTCP:LISTEN >/dev/null 2>&1); then
    echo "FAIL: port ${port} already bound; smoke needs it free \
(override with SMOKE_GATEWAY_PORT / SMOKE_SPA_PORT)" >&2
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

GATEWAY_URL="${SMOKE_GATEWAY_URL:-http://localhost:${SMOKE_GATEWAY_PORT}}"
SPA_URL="${SMOKE_SPA_URL:-http://localhost:${SMOKE_SPA_PORT}}"

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

# --- record the green run -----------------------------------------------------
# Refresh LAST-PASS.md so a PASS leaves committed, machine-readable evidence
# (the DEC-029 "authored + builds != works" gate). Best-effort: never let the
# evidence write flip a green run red.
record_last_pass() {
  local out="${SMOKE_DIR}/LAST-PASS.md"
  local stamp gw_digest web_digest
  stamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  gw_digest="$(docker image inspect argo:dev --format '{{.Id}}' 2>/dev/null || echo 'unknown')"
  web_digest="$(docker image inspect argo-web:dev --format '{{.Id}}' 2>/dev/null || echo 'unknown')"
  {
    echo "# deploy/smoke — LAST PASS"
    echo
    echo "Auto-written by \`deploy/smoke/run.sh\` on every green run. Do NOT edit by"
    echo "hand — it is the recorded evidence that the smoke stack actually came up"
    echo "and served its endpoints on a real machine (DEC-029: \"authored + builds\""
    echo "is not \"works\")."
    echo
    echo "- **Last green run (UTC):** ${stamp}"
    echo "- **Host:** $(uname -srm 2>/dev/null || echo unknown)"
    echo "- **Gateway image (argo:dev) id:** \`${gw_digest}\`"
    echo "- **Web image (argo-web:dev) id:** \`${web_digest}\`"
    echo "- **Gateway URL polled:** ${GATEWAY_URL}/health (200)"
    echo "- **SPA URL polled:** ${SPA_URL}/ (200)"
    echo
    echo "## checks.ts output (all \`\"ok\":true\`)"
    echo
    echo '```json'
    cat checks.out
    echo '```'
  } > "${out}"
  echo "--- recorded evidence to ${out} ---"
}
record_last_pass || echo "WARN: record_last_pass failed (smoke still PASS)" >&2

echo "--- smoke PASS ---"
