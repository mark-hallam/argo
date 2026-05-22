#!/usr/bin/env bash
# Smoke tests for scripts/restart-live-gateway.sh.
#
# Three scenarios:
#   (1) No gateway running        → script starts the fake binary,
#                                   confirms /readyz, reports new version.
#   (2) Gateway running + current → script no-ops with
#                                   "already at schema_version N".
#   (3) Gateway running + stale   → script kills the stale fake, rebuild
#                                   skipped because sidecar matches HEAD,
#                                   launches fresh fake, confirms new
#                                   schema_version >= expected.
#
# A fake "argo" binary serves a tiny /readyz HTTP endpoint via Python,
# reading its schema_version from a file. We point ARGO_RESTART_BIN at
# it and pin ARGO_HOME / CARGO_TARGET_DIR to a tempdir so the test
# never touches the operator's real env.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"
[[ -n "$REPO_ROOT" ]] || { echo "must run inside argo git repo" >&2; exit 1; }
cd "$REPO_ROOT"

SCRIPT="$REPO_ROOT/scripts/restart-live-gateway.sh"
[[ -x "$SCRIPT" ]] || { echo "script not executable: $SCRIPT" >&2; exit 1; }

EXPECTED="$(
    grep -E '^pub const EXPECTED_SCHEMA_VERSION: u32' \
        crates/argo-store/src/lib.rs \
        | sed -E 's/.*= *([0-9]+).*/\1/'
)"
[[ -n "$EXPECTED" ]] || { echo "could not read EXPECTED_SCHEMA_VERSION" >&2; exit 1; }
STALE=$((EXPECTED - 1))

# ---------- harness setup --------------------------------------------

TMPDIR="$(mktemp -d -t restart-live-gateway-smoke.XXXXXX)"
trap 'cleanup' EXIT

PORT=$(( (RANDOM % 5000) + 41000 ))
FAKE_BIN="$TMPDIR/argo"
VER_FILE="$TMPDIR/schema_version"
PID_FILE="$TMPDIR/fake.pid"
LOG_FILE="$TMPDIR/fake.log"

# Pre-set the sidecar so the script doesn't try to cargo build.
export ARGO_HOME="$TMPDIR/argo-home"
mkdir -p "$ARGO_HOME"
git rev-parse HEAD > "$ARGO_HOME/gateway-built-from"

export CARGO_TARGET_DIR="$TMPDIR/cargo-target"
export ARGO_RESTART_BIN="$FAKE_BIN"

# Find a free port (PORT may be in use; bump up to 10 times).
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

cleanup() {
    # Kill any fake gateway we launched (via PID file + by port).
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

# Create the fake "argo" binary. It accepts `gateway start --bind X
# --port Y` (matching the real CLI surface) and serves /readyz from a
# version file. It writes its own PID to PID_FILE.
cat > "$FAKE_BIN" <<EOF
#!/usr/bin/env bash
# Fake argo binary for restart-live-gateway smoke tests.
# Recognized invocation: argo gateway start --bind HOST --port PORT
[[ "\$1" == "gateway" && "\$2" == "start" ]] || { echo "fake-argo: unsupported args: \$*" >&2; exit 2; }

HOST="127.0.0.1"
PORT_ARG=7400
while [[ \$# -gt 0 ]]; do
    case "\$1" in
        --bind) HOST="\$2"; shift 2 ;;
        --port) PORT_ARG="\$2"; shift 2 ;;
        *) shift ;;
    esac
done

VER_FILE="$VER_FILE"
PID_FILE="$PID_FILE"

# Serve /readyz with a schema_version captured at boot (mimics how a
# real gateway returns a fixed version per process — not live-reloaded
# from disk).
python3 -u - "\$HOST" "\$PORT_ARG" "\$VER_FILE" "\$PID_FILE" <<'PY'
import os, sys
from http.server import HTTPServer, BaseHTTPRequestHandler
host, port, ver_file, pid_file = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4]
with open(pid_file, "w") as f:
    f.write(str(os.getpid()))
BOOT_VER = int(open(ver_file).read().strip())
class H(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/readyz":
            body = ('{"status":"ok","version":"fake","schema_version":'
                    + str(BOOT_VER) + ',"uptime_seconds":1}').encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        else:
            self.send_response(404); self.end_headers()
    def log_message(self, *a, **kw): pass
HTTPServer((host, port), H).serve_forever()
PY
EOF
chmod +x "$FAKE_BIN"

start_fake_at_version() {
    local v="$1"
    echo "$v" > "$VER_FILE"
    nohup "$FAKE_BIN" gateway start --bind 127.0.0.1 --port "$PORT" \
        >"$LOG_FILE" 2>&1 < /dev/null &
    disown 2>/dev/null || true
    # Wait up to 5s for the listener to come up.
    for _ in $(seq 1 10); do
        if curl -fsS --max-time 1 "http://127.0.0.1:$PORT/readyz" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.5
    done
    echo "fake gateway failed to start at version $v" >&2
    cat "$LOG_FILE" >&2 || true
    return 1
}

stop_fake() {
    if port_pid=$(lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t 2>/dev/null | head -n1); then
        kill -TERM "$port_pid" 2>/dev/null || true
        for _ in $(seq 1 10); do
            kill -0 "$port_pid" 2>/dev/null || return 0
            sleep 0.3
        done
        kill -KILL "$port_pid" 2>/dev/null || true
    fi
    rm -f "$PID_FILE"
}

assert_version() {
    local want="$1" got
    got="$(curl -fsS --max-time 2 "http://127.0.0.1:$PORT/readyz" \
        | sed -E 's/.*"schema_version" *: *([0-9]+).*/\1/')"
    if [[ "$got" != "$want" ]]; then
        echo "expected schema_version=$want, got=$got" >&2
        return 1
    fi
}

pass() { printf '  ✅ %s\n' "$*"; }
fail() { printf '  ❌ %s\n' "$*" >&2; exit 1; }

run_script() {
    "$SCRIPT" --no-prompt --port "$PORT" --host 127.0.0.1
}

# ---------- scenario 1: stopped --------------------------------------

echo "Scenario 1: no gateway running → script starts a fresh one."
stop_fake
echo "$EXPECTED" > "$VER_FILE"
if out=$(run_script 2>&1); then
    pass "exit 0"
else
    echo "$out" >&2
    fail "script exited non-zero on stopped scenario"
fi
echo "$out" | grep -q "Gateway live at schema_version $EXPECTED" \
    || { echo "$out" >&2; fail "missing 'Gateway live at schema_version $EXPECTED' line"; }
echo "$out" | grep -q "was stopped" \
    || { echo "$out" >&2; fail "missing 'was stopped' diff line"; }
assert_version "$EXPECTED" && pass "post-restart /readyz returns $EXPECTED"
stop_fake

# ---------- scenario 2: current --------------------------------------

echo "Scenario 2: gateway already at EXPECTED → no-op."
start_fake_at_version "$EXPECTED"
if out=$(run_script 2>&1); then
    pass "exit 0"
else
    echo "$out" >&2
    fail "script exited non-zero on current scenario"
fi
echo "$out" | grep -q "already at schema_version $EXPECTED (current). No-op." \
    || { echo "$out" >&2; fail "missing 'already at schema_version' no-op line"; }
assert_version "$EXPECTED" && pass "/readyz still returns $EXPECTED post-no-op"
stop_fake

# ---------- scenario 3: stale ----------------------------------------

echo "Scenario 3: gateway at STALE → kill + relaunch + new version."
start_fake_at_version "$STALE"
assert_version "$STALE" || fail "fake didn't start at $STALE"
# Bump the version file so the relaunched fake reports EXPECTED.
echo "$EXPECTED" > "$VER_FILE"
if out=$(run_script 2>&1); then
    pass "exit 0"
else
    echo "$out" >&2
    fail "script exited non-zero on stale scenario"
fi
echo "$out" | grep -q "Gateway live at schema_version $EXPECTED (was $STALE)" \
    || { echo "$out" >&2; fail "missing 'was $STALE' diff line"; }
assert_version "$EXPECTED" && pass "/readyz returns $EXPECTED after restart"
stop_fake

echo "✅ All 3 scenarios passed."
