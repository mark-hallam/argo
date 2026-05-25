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
#         ./scripts/demo-dry-run.sh --demo-mode
#
# --demo-mode is the branch for actual v1.0 recording sessions. It:
#   * Validates SKYCOT_DB_KEY_BASE64 is exported BEFORE any gateway
#     boot or boot-equivalent step. If missing, exits non-zero with
#     an `openssl rand -base64 32` remediation in stderr.
#   * Asserts POST /api/v1/demo/mint-principal on the configured
#     gateway returns { ok: true, principal: "...", session_token: "..." }.
#   * Does NOT touch macOS Keychain, recover an existing DB key, or
#     prompt for a local password. Recording uses a fresh throwaway
#     SKYCOT_DB_KEY_BASE64 for a fresh demo store.
#
# Non-demo-mode behaviour (the default) is unchanged.
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
DEMO_MODE=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --keep-server) KEEP_SERVER=1; shift ;;
    --gateway) GATEWAY="$2"; shift 2 ;;
    --demo-mode) DEMO_MODE=1; shift ;;
    -h|--help) sed -n '2,42p' "$0"; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; exit 2 ;;
  esac
done

# --- 0. Demo-mode pre-flight (runs BEFORE any gateway-boot or stub launch).
# The brief contract: with --demo-mode, SKYCOT_DB_KEY_BASE64 must be
# exported before anything that could touch a gateway, including the
# Slack stub or the SPA dev server (which inherit the env). Failure
# here is non-zero exit with a clear remediation in stderr.
if [[ ${DEMO_MODE} -eq 1 ]]; then
  if [[ -z "${SKYCOT_DB_KEY_BASE64:-}" ]]; then
    {
      echo "FAIL: --demo-mode requires SKYCOT_DB_KEY_BASE64 to be set"
      echo "      BEFORE any gateway boot or boot-equivalent step."
      echo ""
      echo "      The recording-session path uses a fresh throwaway"
      echo "      32-byte secret for a fresh demo store — it does NOT"
      echo "      read the macOS Keychain, recover an existing DB"
      echo "      key, or prompt for a local password."
      echo ""
      echo "      Generate one and re-run:"
      echo ""
      echo "      export SKYCOT_DB_KEY_BASE64=\"\$(openssl rand -base64 32)\""
    } >&2
    exit 2
  fi
  echo ">> demo-mode: SKYCOT_DB_KEY_BASE64 present (${#SKYCOT_DB_KEY_BASE64} chars)"
fi

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
#
# Per `mission-demo-seed-data-bootstrap`: the SPA must talk to the
# real seeded backend, NOT the mock-bypass path. We deliberately do
# NOT set `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` here — the
# operator runs `argo demo seed --tenant <id>` before the recording
# session so /team / /audit / /workflows render real DB rows.
# `--demo-mode` still sets `ARGO_GATEWAY_AUTH_BYPASS=1` on the
# GATEWAY side (a distinct env var) so the operator-only
# `/api/v1/demo/mint-principal` route resolves; that's an audit
# bypass, not a SPA mock bypass.
if [[ ${KEEP_SERVER} -eq 0 ]]; then
  echo ">> launching next dev on :3000 (real-backend mode, no SPA auth-bypass)"
  ( cd "${WEB_DIR}" \
    && NEXT_PUBLIC_GATEWAY_URL="${GATEWAY}" \
       SLACK_API_BASE_URL="http://127.0.0.1:${SLACK_STUB_PORT}" \
       pnpm dev >"${OUT_DIR}/dev.log" 2>&1 ) &
  SERVER_PID=$!
  for _ in $(seq 1 60); do
    code=$(curl -s -o /dev/null -w "%{http_code}" "${SPA}" || echo 000)
    [[ "${code}" =~ ^(200|307|308)$ ]] && { echo ">> dev server up"; break; }
    sleep 2
  done
fi

# --- 2b. Marketing dev server (port 3001) — scene 1 needs this for the
# "Hire your first worker" CTA, which lives in `marketing/`, not in the SPA.
MARKETING_DIR="${REPO_ROOT}/marketing"
MARKETING="${MARKETING_URL:-http://localhost:3001}"
if [[ ${KEEP_SERVER} -eq 0 && -d "${MARKETING_DIR}" ]]; then
  echo ">> launching marketing dev on :3001"
  ( cd "${MARKETING_DIR}" && pnpm dev >"${OUT_DIR}/marketing.log" 2>&1 ) &
  MARKETING_PID=$!
  for _ in $(seq 1 60); do
    code=$(curl -s -o /dev/null -w "%{http_code}" "${MARKETING}" || echo 000)
    [[ "${code}" =~ ^(200|307|308)$ ]] && { echo ">> marketing dev up"; break; }
    sleep 2
  done
fi

# --- 2c. Demo-mode mint-principal probe. Run AFTER the gateway is
# reachable (we treat ${GATEWAY} as the live endpoint here) but
# BEFORE the playwright walker. Validates that the operator-only
# audit-bypass route is mounted and returns the documented envelope.
if [[ ${DEMO_MODE} -eq 1 ]]; then
  echo ">> demo-mode: probing POST ${GATEWAY}/api/v1/demo/mint-principal"
  MINT_BODY='{"email":"demo+dryrun@throwaway.cc"}'
  MINT_RESP="$(curl -sS -X POST \
    -H 'content-type: application/json' \
    -d "${MINT_BODY}" \
    "${GATEWAY}/api/v1/demo/mint-principal" 2>&1)" || {
      echo "FAIL: --demo-mode: curl to ${GATEWAY}/api/v1/demo/mint-principal failed: ${MINT_RESP}" >&2
      exit 1
    }
  echo "${MINT_RESP}" | jq -e '.ok == true and (.principal | type == "string") and (.session_token | type == "string")' >/dev/null 2>&1 || {
    {
      echo "FAIL: --demo-mode: /api/v1/demo/mint-principal response did not match {ok:true, principal:string, session_token:string}."
      echo "      Response was:"
      echo "      ${MINT_RESP}"
      echo ""
      echo "      Check that the gateway booted with both"
      echo "      ARGO_DEMO_MODE=1 AND ARGO_GATEWAY_AUTH_BYPASS=1."
    } >&2
    exit 1
  }
  echo ">> demo-mode: mint-principal probe GREEN"
fi

# --- 3. Playwright scene walker.
SPEC="${WEB_DIR}/dry-run.mjs"
cp "${FIXTURES}/dry-run.mjs" "${SPEC}"

echo ">> walking demo scenes via Playwright"
SCENES_JSON="${RESULTS}.scenes"
STUB_LOG="${OUT_DIR}/.slack-stub.log"
export SPA GATEWAY FIXTURES MARKETING
export OUT_DIR RESULTS_FILE="${SCENES_JSON}" SLACK_STUB_LOG="${STUB_LOG}"
export DEMO_MODE
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
