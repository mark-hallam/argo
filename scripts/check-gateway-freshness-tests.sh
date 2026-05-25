#!/usr/bin/env bash
# Smoke tests for scripts/check-gateway-freshness.sh.
#
# The script exposes FAKE_HEALTH_BODY / FAKE_HEALTH_STATUS test hooks so
# the smoke harness can exercise HTTP 200 / 503 / malformed bodies
# without binding a local test server.
#
# Five scenarios, covering the documented exit codes plus HTTP 503 stale:
#   (0) drift=0 → exit 0 (FRESH)
#   (1) drift=3 → exit 1 (STALE)
#   (2) /health returns 503 with schema_version → exit 1 (STALE-503)
#   (3) port has no listener → exit 2 (UNREACHABLE)
#   (4) /health returns malformed JSON → exit 3 (PARSE_ERROR)

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"
[[ -n "$REPO_ROOT" ]] || { echo "must run inside argo git repo" >&2; exit 1; }
cd "$REPO_ROOT"

SCRIPT="$REPO_ROOT/scripts/check-gateway-freshness.sh"
[[ -x "$SCRIPT" ]] || { echo "script not executable: $SCRIPT" >&2; exit 1; }

# Read codebase_latest the same way the script does so the test
# matches whatever migrations are on HEAD right now.
# shellcheck disable=SC2012
CODEBASE_LATEST_N="$(
    ls crates/argo-store/src/migrations/v*.rs \
        | sort -V \
        | tail -1 \
        | grep -oE 'v[0-9]+' \
        | head -1 \
        | tr -d v
)"
[[ -n "$CODEBASE_LATEST_N" ]] || { echo "no migrations found on disk" >&2; exit 1; }

PORT=$(( (RANDOM % 5000) + 41000 ))

find_free_port() {
    local p=$PORT
    for _ in $(seq 1 10); do
        if ! lsof -nP -iTCP:"$p" -sTCP:LISTEN -t >/dev/null 2>&1; then
            PORT="$p"
            return 0
        fi
        p=$((p + 1))
    done
    echo "could not find free port near $PORT" >&2
    return 1
}
find_free_port

schema_body() {
    local version="$1"
    printf '{"status":"ok","version":"fake","schema_version":%s,"uptime_seconds":1}' "$version"
}

schema_503_body() {
    local version="$1"
    printf '{"status":"degraded","reason":"schema_version_mismatch","schema_version":%s,"uptime_seconds":1}' "$version"
}

garbage_body() {
    printf '{"status":"ok","version":"fake","oops":"no schema key"}'
}

run_fake_check() {
    local status="$1" body="$2"
    set +e
    OUT="$(FAKE_HEALTH_STATUS="$status" FAKE_HEALTH_BODY="$body" "$SCRIPT" "http://127.0.0.1:$PORT" 2>&1)"
    RC=$?
    set -e
}

run_unreachable_check() {
    set +e
    OUT="$("$SCRIPT" "http://127.0.0.1:$PORT" 2>&1)"
    RC=$?
    set -e
}

pass() { printf '  ✅ %s\n' "$*"; }
fail() { printf '  ❌ %s\n' "$*" >&2; echo "  output was:" >&2; printf '%s\n' "$OUT" >&2; exit 1; }

# ---------- scenario 0: FRESH (drift=0) -----------------------------

echo "Scenario 0: gateway schema == codebase latest → exit 0 (FRESH)."
run_fake_check 200 "$(schema_body "$CODEBASE_LATEST_N")"
[[ $RC -eq 0 ]] || fail "expected exit 0, got $RC"
echo "$OUT" | grep -q "gateway_schema=v${CODEBASE_LATEST_N} codebase_latest=v${CODEBASE_LATEST_N} drift=0" \
    || fail "missing machine-grepable line"
echo "$OUT" | grep -q "STATUS: FRESH" || fail "missing STATUS: FRESH"
pass "drift=0 → exit 0 + FRESH"

# ---------- scenario 1: STALE (drift=3) -----------------------------

STALE_N=$(( CODEBASE_LATEST_N - 3 ))
echo "Scenario 1: gateway 3 behind codebase → exit 1 (STALE)."
run_fake_check 200 "$(schema_body "$STALE_N")"
[[ $RC -eq 1 ]] || fail "expected exit 1, got $RC"
echo "$OUT" | grep -q "gateway_schema=v${STALE_N} codebase_latest=v${CODEBASE_LATEST_N} drift=3" \
    || fail "missing machine-grepable line with drift=3"
echo "$OUT" | grep -q "STATUS: STALE" || fail "missing STATUS: STALE"
pass "drift=3 → exit 1 + STALE"

# ---------- scenario 2: STALE-503 -----------------------------------

STALE_503_N=$(( CODEBASE_LATEST_N - 1 ))
echo "Scenario 2: /health returns HTTP 503 with schema_version → exit 1 (STALE-503)."
run_fake_check 503 "$(schema_503_body "$STALE_503_N")"
[[ $RC -eq 1 ]] || fail "expected exit 1, got $RC"
echo "$OUT" | grep -q "gateway_schema=v${STALE_503_N} codebase_latest=v${CODEBASE_LATEST_N} drift=1" \
    || fail "missing machine-grepable line with drift=1"
echo "$OUT" | grep -q "STATUS: STALE-503" || fail "missing STATUS: STALE-503"
pass "HTTP 503 + schema_version → exit 1 + STALE-503"

# ---------- scenario 3: UNREACHABLE ---------------------------------

echo "Scenario 3: nothing listening on \$PORT → exit 2 (UNREACHABLE)."
run_unreachable_check
[[ $RC -eq 2 ]] || fail "expected exit 2, got $RC"
echo "$OUT" | grep -q "STATUS: UNREACHABLE" || fail "missing STATUS: UNREACHABLE"
pass "no listener → exit 2 + UNREACHABLE"

# ---------- scenario 4: PARSE_ERROR ---------------------------------

echo "Scenario 4: /health returns body without schema_version → exit 3."
run_fake_check 200 "$(garbage_body)"
[[ $RC -eq 3 ]] || fail "expected exit 3, got $RC"
echo "$OUT" | grep -q "STATUS: PARSE_ERROR" || fail "missing STATUS: PARSE_ERROR"
pass "no schema_version in body → exit 3 + PARSE_ERROR"

echo "✅ All 5 scenarios passed."
