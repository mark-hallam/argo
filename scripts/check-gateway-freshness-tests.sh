#!/usr/bin/env bash
# Smoke tests for scripts/check-gateway-freshness.sh.
#
# A Python fake gateway serves /health with a chosen schema_version
# (or a chosen body for the parse-error case). The script under test
# is pointed at the fake via its first positional argument
# (`http://127.0.0.1:PORT`).
#
# Four scenarios, one per documented exit code:
#   (0) drift=0 → exit 0 (FRESH)
#   (1) drift=3 → exit 1 (STALE)
#   (2) port has no listener → exit 2 (UNREACHABLE)
#   (3) /health returns malformed JSON → exit 3 (PARSE_ERROR)

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

TMPDIR="$(mktemp -d -t check-gateway-freshness-smoke.XXXXXX)"
PID_FILE="$TMPDIR/fake.pid"
LOG_FILE="$TMPDIR/fake.log"
PORT=$(( (RANDOM % 5000) + 41000 ))

cleanup() {
    if [[ -f "$PID_FILE" ]]; then
        local pid
        pid="$(cat "$PID_FILE" 2>/dev/null || true)"
        [[ -n "$pid" ]] && kill -KILL "$pid" 2>/dev/null || true
    fi
    if port_pid=$(lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t 2>/dev/null | head -n1); then
        [[ -n "${port_pid:-}" ]] && kill -KILL "$port_pid" 2>/dev/null || true
    fi
    rm -rf "$TMPDIR"
}
trap cleanup EXIT

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

# Start a fake /health endpoint. MODE=schema serves a normal JSON body
# with the supplied schema_version; MODE=garbage serves a body without
# the expected key so the script's parser must fail with exit 3.
start_fake() {
    local mode="$1" version="${2:-0}"
    PORT_ARG="$PORT" MODE="$mode" VER="$version" PID_FILE="$PID_FILE" \
        nohup python3 -u -c '
import os, sys
from http.server import HTTPServer, BaseHTTPRequestHandler
port = int(os.environ["PORT_ARG"])
mode = os.environ["MODE"]
ver  = os.environ["VER"]
with open(os.environ["PID_FILE"], "w") as f:
    f.write(str(os.getpid()))
class H(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/health":
            if mode == "schema":
                body = ("{\"status\":\"ok\",\"version\":\"fake\","
                        "\"schema_version\":" + ver + ",\"uptime_seconds\":1}").encode()
            else:
                body = b"{\"status\":\"ok\",\"version\":\"fake\",\"oops\":\"no schema key\"}"
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        else:
            self.send_response(404); self.end_headers()
    def log_message(self, *a, **kw): pass
HTTPServer(("127.0.0.1", port), H).serve_forever()
' >"$LOG_FILE" 2>&1 < /dev/null &
    disown 2>/dev/null || true
    for _ in $(seq 1 10); do
        if curl -fsS --max-time 1 "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.3
    done
    echo "fake gateway failed to start" >&2
    cat "$LOG_FILE" >&2 || true
    return 1
}

stop_fake() {
    if port_pid=$(lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t 2>/dev/null | head -n1); then
        kill -TERM "$port_pid" 2>/dev/null || true
        for _ in $(seq 1 10); do
            kill -0 "$port_pid" 2>/dev/null || return 0
            sleep 0.2
        done
        kill -KILL "$port_pid" 2>/dev/null || true
    fi
    rm -f "$PID_FILE"
}

run_check() {
    set +e
    OUT="$("$SCRIPT" "http://127.0.0.1:$PORT" 2>&1)"
    RC=$?
    set -e
}

pass() { printf '  ✅ %s\n' "$*"; }
fail() { printf '  ❌ %s\n' "$*" >&2; echo "  output was:" >&2; printf '%s\n' "$OUT" >&2; exit 1; }

# ---------- scenario 0: FRESH (drift=0) -----------------------------

echo "Scenario 0: gateway schema == codebase latest → exit 0 (FRESH)."
start_fake schema "$CODEBASE_LATEST_N"
run_check
[[ $RC -eq 0 ]] || fail "expected exit 0, got $RC"
echo "$OUT" | grep -q "gateway_schema=v${CODEBASE_LATEST_N} codebase_latest=v${CODEBASE_LATEST_N} drift=0" \
    || fail "missing machine-grepable line"
echo "$OUT" | grep -q "STATUS: FRESH" || fail "missing STATUS: FRESH"
pass "drift=0 → exit 0 + FRESH"
stop_fake

# ---------- scenario 1: STALE (drift=3) -----------------------------

STALE_N=$(( CODEBASE_LATEST_N - 3 ))
echo "Scenario 1: gateway 3 behind codebase → exit 1 (STALE)."
start_fake schema "$STALE_N"
run_check
[[ $RC -eq 1 ]] || fail "expected exit 1, got $RC"
echo "$OUT" | grep -q "gateway_schema=v${STALE_N} codebase_latest=v${CODEBASE_LATEST_N} drift=3" \
    || fail "missing machine-grepable line with drift=3"
echo "$OUT" | grep -q "STATUS: STALE" || fail "missing STATUS: STALE"
pass "drift=3 → exit 1 + STALE"
stop_fake

# ---------- scenario 2: UNREACHABLE ---------------------------------

echo "Scenario 2: nothing listening on \$PORT → exit 2 (UNREACHABLE)."
stop_fake  # belt-and-braces
run_check
[[ $RC -eq 2 ]] || fail "expected exit 2, got $RC"
echo "$OUT" | grep -q "STATUS: UNREACHABLE" || fail "missing STATUS: UNREACHABLE"
pass "no listener → exit 2 + UNREACHABLE"

# ---------- scenario 3: PARSE_ERROR ---------------------------------

echo "Scenario 3: /health returns body without schema_version → exit 3."
start_fake garbage
run_check
[[ $RC -eq 3 ]] || fail "expected exit 3, got $RC"
echo "$OUT" | grep -q "STATUS: PARSE_ERROR" || fail "missing STATUS: PARSE_ERROR"
pass "no schema_version in body → exit 3 + PARSE_ERROR"
stop_fake

echo "✅ All 4 scenarios passed."
