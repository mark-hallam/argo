#!/usr/bin/env bash
# Dry-run for the v1.0 mission demo (docs/skycot/demos/v1-mission-mvp.mp4).
#
# Walks the demo flow against a local SPA + gateway with Playwright +
# curl + jq, emits per-scene screenshots and a JSON results file, and
# exits non-zero on the first broken scene. Operator runs this the
# night before recording; if green, recording can proceed.
#
# Prereqs (one-time):  cd web && pnpm install
#                      cd web && pnpm exec playwright install chromium
#                      brew install jq
#
# Usage:  ./scripts/demo-dry-run.sh
#         ./scripts/demo-dry-run.sh --keep-server
#         ./scripts/demo-dry-run.sh --gateway http://127.0.0.1:8787
#
# What it verifies (one row per acceptance bullet in the brief):
#   1. marketing CTA reachable, has "Hire your first worker" text
#   2. signup endpoint accepts a throwaway email (200/204)
#   3. magic-link verify route renders without 404
#   4. all five onboarding wizard pages render with "Next" / "Get started"
#   5. workers POST endpoint accepts the first-hire payload (200/201)
#   6. ApprovalAsked event appears in audit chain after first hire
#   7. Slack stub webhook receives the channel-notify (env-routed)
#   8. /audit renders the chain with non-empty rows
#
# Slack delivery is verified against a local stub (an HTTP listener on
# :4455) rather than the real Slack API; SLACK_API_BASE_URL is routed
# at the listener for the duration of the run.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEB_DIR="${REPO_ROOT}/web"
FIXTURES="${REPO_ROOT}/scripts/demo-dry-run-fixtures"
OUT_DIR="${TMPDIR:-/tmp}/skycot-demo-dry-run"
TODAY="$(date -u +%Y-%m-%d)"
RESULTS="${OUT_DIR}/dry-run-results-${TODAY}.json"
GATEWAY="${GATEWAY_URL:-http://127.0.0.1:8787}"
SPA="http://localhost:3000"
SLACK_STUB_PORT=4455

KEEP_SERVER=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --keep-server) KEEP_SERVER=1; shift ;;
    --gateway) GATEWAY="$2"; shift 2 ;;
    -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; exit 2 ;;
  esac
done

command -v pnpm >/dev/null 2>&1 || { echo "FAIL: pnpm not on PATH" >&2; exit 2; }
command -v jq   >/dev/null 2>&1 || { echo "FAIL: jq not on PATH (brew install jq)" >&2; exit 2; }
command -v curl >/dev/null 2>&1 || { echo "FAIL: curl not on PATH" >&2; exit 2; }
[[ -d "${FIXTURES}" ]] || { echo "FAIL: fixtures dir missing: ${FIXTURES}" >&2; exit 2; }

mkdir -p "${OUT_DIR}"
echo "[]" >"${RESULTS}.scenes"

SERVER_PID=""
STUB_PID=""
# shellcheck disable=SC2329  # invoked via `trap cleanup EXIT` below
cleanup() {
  rm -f "${WEB_DIR}/dry-run.mjs" "${RESULTS}.scenes" "${OUT_DIR}/.slack-stub.log" 2>/dev/null || true
  [[ -n "${STUB_PID}" ]]   && kill "${STUB_PID}"   2>/dev/null || true
  [[ -n "${SERVER_PID}" && ${KEEP_SERVER} -eq 0 ]] && kill "${SERVER_PID}" 2>/dev/null || true
}
trap cleanup EXIT

# --- 1. Slack stub: tiny Python HTTP listener that records every POST.
echo ">> launching Slack stub on :${SLACK_STUB_PORT}"
python3 "${FIXTURES}/slack_stub.py" "${SLACK_STUB_PORT}" "${OUT_DIR}/.slack-stub.log" &
STUB_PID=$!
for _ in $(seq 1 10); do
  curl -s -o /dev/null "http://127.0.0.1:${SLACK_STUB_PORT}/healthz" && break
  sleep 0.5
done

# --- 2. SPA dev server (with gateway pointed at the real-or-stub backend).
if [[ ${KEEP_SERVER} -eq 0 ]]; then
  echo ">> launching next dev on :3000"
  ( cd "${WEB_DIR}" \
    && NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 \
       NEXT_PUBLIC_GATEWAY_URL="${GATEWAY}" \
       SLACK_API_BASE_URL="http://127.0.0.1:${SLACK_STUB_PORT}" \
       pnpm dev >"${OUT_DIR}/dev.log" 2>&1 ) &
  SERVER_PID=$!
  for _ in $(seq 1 60); do
    code=$(curl -s -o /dev/null -w "%{http_code}" "${SPA}" || echo 000)
    [[ "${code}" =~ ^(200|307|308)$ ]] && { echo ">> dev server up"; break; }
    sleep 2
  done
fi

# --- 3. Playwright scene walker.
SPEC="${WEB_DIR}/dry-run.mjs"
cp "${FIXTURES}/dry-run.mjs" "${SPEC}"

echo ">> walking demo scenes via Playwright"
SCENES_JSON="${RESULTS}.scenes"
STUB_LOG="${OUT_DIR}/.slack-stub.log"
export SPA GATEWAY FIXTURES
export OUT_DIR RESULTS_FILE="${SCENES_JSON}" SLACK_STUB_LOG="${STUB_LOG}"
( cd "${WEB_DIR}" && RESULTS="${RESULTS_FILE}" \
    node --experimental-vm-modules "${SPEC}" )
walker_rc=$?

# --- 4. Assemble final results JSON.
jq -n \
  --arg date "${TODAY}" \
  --arg gateway "${GATEWAY}" \
  --arg spa "${SPA}" \
  --slurpfile scenes "${RESULTS}.scenes" \
  '{date:$date, gateway:$gateway, spa:$spa, scenes:$scenes[0]}' \
  >"${RESULTS}"

red=$(jq -r '[.scenes[] | select(.status!="pass")] | length' "${RESULTS}")
total=$(jq -r '.scenes | length' "${RESULTS}")
echo ">> results: ${RESULTS}"
echo ">> ${total} scenes, $((total - red)) pass, ${red} fail"

if [[ "${red}" -gt 0 || "${walker_rc}" -ne 0 ]]; then
  echo ">> per-scene failures:"
  jq -r '.scenes[] | select(.status!="pass") | "  - \(.scene): \(.error)"' "${RESULTS}"
  exit 1
fi

echo ">> dry-run GREEN — recording can proceed."
exit 0
