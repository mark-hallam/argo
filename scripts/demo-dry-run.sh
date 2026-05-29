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
#         ./scripts/demo-dry-run.sh --gateway http://127.0.0.1:7400
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
GATEWAY="${GATEWAY_URL:-http://127.0.0.1:7400}"
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

# --- 0b. Demo-mode launch.json env-triplet gate.
# `.claude/launch.json` is gitignored (per-machine operator config), but
# its `web` runtime command is what `claude preview start web` executes
# on recording day. The recording-day SPA contract needs all three of:
#  - NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 (canonical name; not the
#    `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` typo a prior brief shipped) —
#    consumers in `web/lib/skycot/demo-mode.ts` read the _ARGO_ variant.
#  - NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 — flips the SPA to seeded mocks /
#    showcase identity in the non-audit-feed paths.
#  - NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1 — opts the audit/approvals/replay
#    clients OUT of the mock short-circuit so Scene 6 hits the real
#    seeded gateway (see web/lib/skycot/demo-mode.ts:48-52).
# Without all three, /audit + /approvals + the replay drawer render
# their empty states on recording day. Fail loud here so the operator
# notices BEFORE pressing record.
if [[ ${DEMO_MODE} -eq 1 ]]; then
  LAUNCH_JSON="${REPO_ROOT}/.claude/launch.json"
  if [[ -f "${LAUNCH_JSON}" ]]; then
    WEB_LINE="$(grep -F 'pnpm --dir web dev' "${LAUNCH_JSON}" || true)"
    if [[ -z "${WEB_LINE}" ]]; then
      {
        echo "FAIL: --demo-mode: ${LAUNCH_JSON} has no 'pnpm --dir web dev'"
        echo "      runtime command. The 'web' configuration that"
        echo "      'claude preview start web' depends on appears missing."
      } >&2
      exit 1
    fi
    triplet_ok=1
    miss=()
    extra=()
    if ! printf '%s' "${WEB_LINE}" | grep -q 'NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1'; then
      miss+=("NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1")
      triplet_ok=0
    fi
    if ! printf '%s' "${WEB_LINE}" | grep -q 'NEXT_PUBLIC_SKYCOT_DEMO_MODE=1'; then
      miss+=("NEXT_PUBLIC_SKYCOT_DEMO_MODE=1")
      triplet_ok=0
    fi
    if ! printf '%s' "${WEB_LINE}" | grep -q 'NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1'; then
      miss+=("NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1")
      triplet_ok=0
    fi
    # Match only the bare typo, not the canonical _ARGO_ form. The leading
    # `[^_]` ensures `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (preceded by
    # `_`) does NOT match — only `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` (the
    # typo, preceded by a space or quote) does.
    if printf '%s' "${WEB_LINE}" | grep -Eq '[^_]NEXT_PUBLIC_GATEWAY_AUTH_BYPASS='; then
      extra+=("NEXT_PUBLIC_GATEWAY_AUTH_BYPASS (typo — rename to NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS)")
      triplet_ok=0
    fi
    if printf '%s' "${WEB_LINE}" | grep -q 'NEXT_PUBLIC_ARGO_DEMO_MODE='; then
      extra+=("NEXT_PUBLIC_ARGO_DEMO_MODE (stale alias — no consumer in web/lib; remove)")
      triplet_ok=0
    fi
    if [[ ${triplet_ok} -ne 1 ]]; then
      {
        echo "FAIL: --demo-mode: ${LAUNCH_JSON} 'web' runtime command is"
        echo "      missing required env vars and/or carries stale ones."
        if [[ ${#miss[@]} -gt 0 ]]; then
          echo "      Missing: ${miss[*]}"
        fi
        if [[ ${#extra[@]} -gt 0 ]]; then
          echo "      Remove:  ${extra[*]}"
        fi
        echo ""
        echo "      Canonical 'web' runtime command:"
        echo "      NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 \\"
        echo "      NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 \\"
        echo "      NEXT_PUBLIC_SKYCOT_DEMO_MODE=1 \\"
        echo "      NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1 \\"
        echo "      pnpm --dir web dev"
      } >&2
      exit 1
    fi
    echo ">> demo-mode: .claude/launch.json env triplet OK"
  else
    echo ">> demo-mode: ${LAUNCH_JSON} not present — skipping triplet check (operator launches SPA out-of-band)"
  fi
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

# --- 2c2. Pre-record roster guard.
#
# Closes scene-1-and-2-claim-1-and-2 F1 [P0]: at audit time the
# gateway was serving a single "Maya" worker against a runbook that
# narrates five names. A roster-shortfall dry-run that PASSES is the
# failure mode that wastes the operator's recording session. Probes
# /api/v1/team for the canonical roster, /api/v1/skills > 0, and
# /api/v1/verticals > 0, and exits non-zero with the seed-CLI command
# verbatim if anything is off. Demo-mode only — non-demo runs hit a
# stub gateway with no seeded rows.
if [[ ${DEMO_MODE} -eq 1 ]]; then
  echo ">> demo-mode: roster + skills + verticals guard against ${GATEWAY}"
  if ! "${REPO_ROOT}/scripts/check-demo-roster.sh" "${GATEWAY}"; then
    {
      echo "FAIL: demo roster guard rejected ${GATEWAY}."
      echo "      Do NOT roll the camera — re-seed and retry."
    } >&2
    exit 1
  fi
fi

# --- 2c-bis. Demo-mode seeded-rows probe. The recording-day SPA's
# Scene 6 hinges on /audit + /approvals returning >0 rows from the
# live gateway. If `argo demo seed --tenant <id>` hasn't been run (or
# ran against a different tenant id, or the gateway booted on a fresh
# encrypted DB after seeding), both feeds render their empty states
# and the "drill into real provenance" narration has nothing to drill
# into. Fail here so the operator re-runs seed before recording.
if [[ ${DEMO_MODE} -eq 1 ]]; then
  AUDIT_COUNT="$(curl -fsS "${GATEWAY}/api/v1/audit?range=7d" 2>/dev/null \
    | jq -r '(.items // []) | length' 2>/dev/null || echo 0)"
  APPROVALS_COUNT="$(curl -fsS "${GATEWAY}/api/v1/approvals" 2>/dev/null \
    | jq -r '(.items // []) | length' 2>/dev/null || echo 0)"
  if [[ "${AUDIT_COUNT}" -le 0 || "${APPROVALS_COUNT}" -le 0 ]]; then
    {
      echo "FAIL: --demo-mode: live gateway has no seeded rows."
      echo "      /api/v1/audit?range=7d returned ${AUDIT_COUNT} items"
      echo "      /api/v1/approvals       returned ${APPROVALS_COUNT} items"
      echo ""
      echo "      Run the canonical seed CLI against the demo tenant"
      echo "      BEFORE pressing record (see v1-mission-mvp-runbook.md"
      echo "      Scene 6 pre-flight checklist):"
      echo ""
      echo "      argo tenant list | grep -i demo"
      echo "      export ARGO_DEMO_TENANT_ID=<uuid-from-demo-row>"
      echo "      argo demo seed --tenant \"\$ARGO_DEMO_TENANT_ID\""
    } >&2
    exit 1
  fi
  echo ">> demo-mode: seeded-rows probe GREEN (audit=${AUDIT_COUNT}, approvals=${APPROVALS_COUNT})"
fi

# --- 2d. Genesis endpoint not 503 (cost-guardrail satisfied).
#
# Closes fix-genesis-orphan-discoverability-and-provider-key-503: the
# investor-recording demo can't proceed if POST /api/v1/genesis fails
# closed because no provider key is exported. The endpoint accepts
# ARGO_GENESIS_PROVIDER_KEY OR ANTHROPIC_API_KEY OR OPENAI_API_KEY; this
# probe sends a deliberately-empty body and asserts the response is NOT
# the 503 `genesis_provider_key_missing` envelope. A 400 (`bad_request`,
# role_description missing) is the expected GREEN signal — it proves
# the env-gate let us through.
echo ">> probing POST ${GATEWAY}/api/v1/genesis env-gate"
GENESIS_RESP="$(curl -sS -X POST \
  -H 'content-type: application/json' \
  -d '{}' \
  "${GATEWAY}/api/v1/genesis" 2>&1)" || true
if printf '%s' "${GENESIS_RESP}" | grep -q 'genesis_provider_key_missing'; then
  {
    echo "FAIL: POST ${GATEWAY}/api/v1/genesis returned the 503 cost-guardrail."
    echo "      Export ARGO_GENESIS_PROVIDER_KEY, ANTHROPIC_API_KEY, or"
    echo "      OPENAI_API_KEY before scripts/restart-live-gateway.sh."
    echo "      Response was:"
    echo "      ${GENESIS_RESP}"
  } >&2
  exit 1
fi
echo ">> Genesis env-gate satisfied (no genesis_provider_key_missing in response)"

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
