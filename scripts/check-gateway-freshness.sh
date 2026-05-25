#!/usr/bin/env bash
# check-gateway-freshness.sh — read-only drift check between a running
# argo gateway and the migrations checked into HEAD.
#
# The operator boots `argo gateway start` once. Days later, new
# migrations and routes land via the swarm. The running process is now
# silently stale — it serves the OLD routes table while the codebase
# has new ones. This script surfaces that drift so the conductor can
# nudge the operator at tick time.
#
# Read-only, non-blocking. The script only reports; it does not restart
# the gateway. `make restart-live-gateway` is the operator's lever.
#
# Inputs:
#   $1 (optional) — gateway URL base. Default: http://127.0.0.1:7400
#
# Probes `<base>/health`, extracts `schema_version` from the JSON body,
# reads the highest-numbered migration slot under
# `crates/argo-store/src/migrations/v*.rs`, and compares.
#
# Output (single machine-grepable line + one human STATUS line):
#   gateway_schema=v70 codebase_latest=v77 drift=7
#   STATUS: STALE  (drift > 2, restart recommended: `make restart-live-gateway`)
#
# Exit codes (documented; conductor uses these as a stable contract):
#   0 — FRESH. Drift <= 2. Operator does not need to act.
#   1 — STALE. Drift > 2, or /health returned HTTP 503 with a
#       schema_version body. Suggest `make restart-live-gateway`.
#   2 — UNREACHABLE. /health did not respond. Gateway likely not
#       running. Distinct from STALE so the conductor doesn't keep
#       nagging when the gateway is intentionally down.
#   3 — PARSE_ERROR. /health responded but the JSON did not contain a
#       `schema_version` integer, or the migrations dir is missing /
#       empty. Either an unexpected /health shape (route changed
#       upstream) or operator error. Distinct from STALE so the
#       conductor flags it as "investigate", not "restart".

set -euo pipefail

BASE_URL="${1:-http://127.0.0.1:7400}"
DRIFT_THRESHOLD=2

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$REPO_ROOT" ]]; then
    echo "STATUS: PARSE_ERROR  (not inside an argo git checkout)" >&2
    exit 3
fi

MIG_DIR="$REPO_ROOT/crates/argo-store/src/migrations"
if [[ ! -d "$MIG_DIR" ]]; then
    echo "STATUS: PARSE_ERROR  (migrations dir missing: $MIG_DIR)" >&2
    exit 3
fi

# Highest migration slot on disk. `sort -V` handles vN ordering (so v77
# wins over v9); `grep -oE 'v[0-9]+'` strips any trailing `_skill_ab_tests`
# style suffix (e.g. v21_skill_ab_tests.rs → v21).
# `ls | sort -V` is the pattern the brief specifies; migration
# filenames are tightly constrained (vN[_slug].rs) so the lexical /
# glob hazards `find` would dodge don't apply here.
# shellcheck disable=SC2012
CODEBASE_LATEST_RAW="$(
    ls "$MIG_DIR"/v*.rs 2>/dev/null \
        | sort -V \
        | tail -1 \
        | grep -oE 'v[0-9]+' \
        | head -1 || true
)"
if [[ -z "$CODEBASE_LATEST_RAW" ]]; then
    echo "STATUS: PARSE_ERROR  (no v*.rs migration files under $MIG_DIR)" >&2
    exit 3
fi
CODEBASE_LATEST_N="${CODEBASE_LATEST_RAW#v}"

# Probe /health. `--max-time 3` keeps the conductor tick snappy even
# when the gateway hangs in some half-up state. Capture the HTTP status
# separately so degraded 503 responses can still be classified from the
# JSON body instead of looking identical to a dead listener.
if [[ -n "${FAKE_HEALTH_BODY+x}" || -n "${FAKE_HEALTH_STATUS+x}" ]]; then
    HEALTH_BODY="${FAKE_HEALTH_BODY:-}"
    HEALTH_STATUS="${FAKE_HEALTH_STATUS:-200}"
else
    HEALTH_RESPONSE="$(curl -sS --max-time 3 -w '\n%{http_code}' "$BASE_URL/health" 2>/dev/null || true)"
    HEALTH_STATUS="${HEALTH_RESPONSE##*$'\n'}"
    HEALTH_BODY="${HEALTH_RESPONSE%$'\n'*}"
fi
if [[ -z "$HEALTH_BODY" ]]; then
    echo "STATUS: UNREACHABLE  (no response from $BASE_URL/health — gateway not running?)" >&2
    exit 2
fi
if [[ ! "$HEALTH_STATUS" =~ ^[0-9]{3}$ ]]; then
    echo "STATUS: UNREACHABLE  (no HTTP status from $BASE_URL/health — gateway not running?)" >&2
    exit 2
fi

# Extract schema_version. The contract from routes_health.rs is
# `"schema_version":N` where N is a bare integer (no quotes). If the
# upstream JSON shape ever changes (quoted, renamed, missing), the sed
# falls through to empty and we exit PARSE_ERROR rather than silently
# claiming "fresh".
GATEWAY_SCHEMA_N="$(
    printf '%s' "$HEALTH_BODY" \
        | sed -nE 's/.*"schema_version"[[:space:]]*:[[:space:]]*([0-9]+).*/\1/p' \
        | head -1
)"
if [[ -z "$GATEWAY_SCHEMA_N" ]]; then
    if [[ "$HEALTH_STATUS" =~ ^5 ]]; then
        echo "STATUS: UNREACHABLE  (/health returned HTTP $HEALTH_STATUS without schema_version)" >&2
        exit 2
    fi
    echo "STATUS: PARSE_ERROR  (/health responded but schema_version not found in JSON)" >&2
    echo "  body: $HEALTH_BODY" >&2
    exit 3
fi

if [[ "$HEALTH_STATUS" == "503" ]]; then
    printf 'gateway_schema=v%s codebase_latest=v%s drift=%s\n' \
        "$GATEWAY_SCHEMA_N" "$CODEBASE_LATEST_N" "$(( CODEBASE_LATEST_N > GATEWAY_SCHEMA_N ? CODEBASE_LATEST_N - GATEWAY_SCHEMA_N : 0 ))"
    # shellcheck disable=SC2016
    printf 'STATUS: STALE-503  (gateway up but reporting schema_version_mismatch - restart binary: `bash scripts/restart-live-gateway.sh`)\n'
    exit 1
fi

if [[ ! "$HEALTH_STATUS" =~ ^2 ]]; then
    echo "STATUS: UNREACHABLE  (/health returned HTTP $HEALTH_STATUS)" >&2
    exit 2
fi

DRIFT=$(( CODEBASE_LATEST_N - GATEWAY_SCHEMA_N ))
# Negative drift (gateway ahead of codebase) is harmless — it means
# main rolled back a migration that the running process already saw.
# Report drift=0 in that case so the math stays sane.
if (( DRIFT < 0 )); then
    DRIFT=0
fi

printf 'gateway_schema=v%s codebase_latest=v%s drift=%s\n' \
    "$GATEWAY_SCHEMA_N" "$CODEBASE_LATEST_N" "$DRIFT"

if (( DRIFT > DRIFT_THRESHOLD )); then
    # Backticks below are part of the human-readable hint, not a
    # command substitution.
    # shellcheck disable=SC2016
    printf 'STATUS: STALE  (drift > %s, restart recommended: `make restart-live-gateway`)\n' \
        "$DRIFT_THRESHOLD"
    exit 1
fi

printf 'STATUS: FRESH  (drift <= %s)\n' "$DRIFT_THRESHOLD"
exit 0
