#!/usr/bin/env bash
# restart-live-gateway.sh — operator-facing safe-restart for the local
# argo gateway. Implements the runnable counterpart of the Option-B
# policy spec (docs/skycot/operator/gateway-restart-policy.md, PR #2023).
#
# Steps:
#   [0/8] First-run bootstrap (if needed): generate SKYCOT_DB_KEY_BASE64,
#         prompt for ANTHROPIC_API_KEY, write $ARGO_HOME/env.local with
#         chmod 600. Then source env.local if it exists.
#   [1/8] Capture old PID + pre-restart schema_version (or "stopped").
#   [2/8] Compare live schema_version against EXPECTED_SCHEMA_VERSION
#         (read from crates/argo-store/src/lib.rs). If they match and
#         a gateway is running, no-op + report (overridden by --force).
#   [3/8] 5s "may briefly interrupt N active integration tests" prompt
#         (skipped with --no-prompt).
#   [4/8] SIGTERM old PID, wait up to 10s, escalate to SIGKILL.
#   [5/8] Rebuild release binary if HEAD has advanced past the binary's
#         "built-from" sidecar OR the binary is missing.
#   [6/8] Launch new gateway detached via nohup.
#   [7/8] Poll /readyz every 1s up to 30s for HTTP 200.
#   [8/8] Confirm new schema_version >= EXPECTED_SCHEMA_VERSION. Print
#         before/after diff.
#
# Binary path is `${CARGO_TARGET_DIR:-./target}/release/argo`.
# Default port 7400 (CLI default); override with --port or
# ARGO_GATEWAY_PORT.
#
# Exits non-zero with a clear diagnostic on any failure step.

set -euo pipefail

# ---------- helpers ---------------------------------------------------

log()  { printf '%s\n' "$*"; }
die()  { printf '❌ %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<'EOF'
Usage: restart-live-gateway.sh [--no-prompt] [--force] [--bootstrap-only]
                               [--port PORT] [--host HOST]

  --no-prompt       Skip the 5s pre-restart warning (for cron/automation).
  --force           Restart even when the live schema_version already
                    matches EXPECTED_SCHEMA_VERSION. Use when switching
                    from a one-time inline key to a persistent
                    $ARGO_HOME/env.local key (the no-op short-circuit
                    otherwise leaves the old in-memory key alive).
  --bootstrap-only  Run first-run bootstrap then exit without restarting.
                    No-op if $ARGO_HOME/env.local already exists.
  --port PORT       Gateway port (default: $ARGO_GATEWAY_PORT or 7400).
  --host HOST       Gateway host (default: 127.0.0.1).
  -h, --help        This message.

Behavior:
  - On first run (no $ARGO_HOME/env.local and SKYCOT_DB_KEY_BASE64
    unset), bootstraps env.local with a fresh 32-byte DB key + API key.
  - Auto-sources $ARGO_HOME/env.local on every run.
  - If no gateway is running, start a fresh one.
  - If a current gateway is running, no-op + report (unless --force).
  - If a stale gateway is running, stop + rebuild-if-needed + restart.
EOF
}

# ---------- arg parsing ----------------------------------------------

NO_PROMPT=0
FORCE=0
BOOTSTRAP_ONLY=0
PORT="${ARGO_GATEWAY_PORT:-7400}"
HOST="127.0.0.1"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-prompt)      NO_PROMPT=1; shift ;;
        --force)          FORCE=1; shift ;;
        --bootstrap-only) BOOTSTRAP_ONLY=1; shift ;;
        --port)           PORT="$2"; shift 2 ;;
        --host)           HOST="$2"; shift 2 ;;
        -h|--help)        usage; exit 0 ;;
        *) die "unknown arg: $1 (try --help)" ;;
    esac
done

# ---------- repo root + paths ----------------------------------------

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
[[ -n "$REPO_ROOT" ]] || die "must run inside the argo git repo"
cd "$REPO_ROOT"

TARGET_DIR="${CARGO_TARGET_DIR:-./target}"
BIN="${ARGO_RESTART_BIN:-$TARGET_DIR/release/argo}"
ARGO_HOME_DIR="${ARGO_HOME:-$HOME/.argo}"
BUILT_FROM_FILE="$ARGO_HOME_DIR/gateway-built-from"
LOG_DIR="$ARGO_HOME_DIR/logs"
ENV_LOCAL_FILE="$ARGO_HOME_DIR/env.local"

HEALTH_URL="http://${HOST}:${PORT}/readyz"

# ---------- step 0: first-run bootstrap + env.local auto-source ------

# Generate a 32-byte SKYCOT_DB_KEY_BASE64 (verified to decode to 32
# bytes), discover or prompt for an LLM API key, write env.local with
# chmod 600. Stdout messages avoid leading '#' so copy-paste into a
# shell never silently swallows lines as comments.
bootstrap_env_local() {
    mkdir -p "$ARGO_HOME_DIR"

    local key length attempts=0
    while :; do
        attempts=$((attempts + 1))
        key="$(openssl rand -base64 32)"
        length="$(printf '%s' "$key" | base64 -d 2>/dev/null | wc -c | tr -d ' ')"
        [[ "$length" == "32" ]] && break
        if (( attempts >= 5 )); then
            die "could not generate a 32-byte base64 key after 5 attempts (got length=$length)"
        fi
    done

    local api_key="" api_var="ANTHROPIC_API_KEY"
    if [[ -n "${ANTHROPIC_API_KEY:-}" ]]; then
        log "      found ANTHROPIC_API_KEY in parent environment; reusing."
        api_key="$ANTHROPIC_API_KEY"
        api_var="ANTHROPIC_API_KEY"
    elif [[ -n "${OPENAI_API_KEY:-}" ]]; then
        log "      found OPENAI_API_KEY in parent environment; reusing."
        api_key="$OPENAI_API_KEY"
        api_var="OPENAI_API_KEY"
    elif [[ -t 0 && "${ARGO_BOOTSTRAP_NONINTERACTIVE:-0}" != "1" ]]; then
        log "      Paste an Anthropic API key (input hidden; blank to skip):"
        read -rs api_key
        printf '\n'
        api_var="ANTHROPIC_API_KEY"
    else
        log "      non-interactive shell and no *_API_KEY in env — writing env.local"
        log "      with a placeholder. Edit $ENV_LOCAL_FILE to add a real key before launch."
    fi

    if [[ -n "$api_key" ]]; then
        if (( ${#api_key} < 20 )) || [[ ! "$api_key" =~ ^(sk-ant-|sk-) ]]; then
            log "      warning: API key looks unusual (length=${#api_key}, prefix='${api_key:0:7}'); writing anyway."
        fi
    fi

    # Write env.local. Note: no leading '#' on stdout messages, but
    # comments inside the file itself are fine — the file is sourced,
    # not pasted into a shell prompt.
    {
        printf '%s\n' '# ~/.argo/env.local — sourced by scripts/restart-live-gateway.sh'
        printf '%s\n' '# Generated by the first-run bootstrap; safe to edit by hand.'
        printf '%s\n' '# chmod 600 enforced — contains a DB encryption key and API key.'
        printf '\n'
        printf '%s\n' '# 32-byte base64 key for the SQLCipher sessions DB. Losing this'
        printf '%s\n' '# means the on-disk sessions.db is unreadable; back it up if you'
        printf '%s\n' '# care about historical sessions.'
        printf "export SKYCOT_DB_KEY_BASE64='%s'\n" "$key"
        printf '\n'
        if [[ -n "$api_key" ]]; then
            printf '%s\n' '# LLM provider key. Rotate by editing this line; in-memory keys'
            printf '%s\n' '# in a running gateway are unchanged until next restart.'
            printf "export %s='%s'\n" "$api_var" "$api_key"
        else
            printf '%s\n' '# LLM provider key — uncomment and fill in before launch.'
            printf '%s\n' "# export ANTHROPIC_API_KEY='sk-ant-...'"
            printf '%s\n' "# export OPENAI_API_KEY='sk-...'"
        fi
    } > "$ENV_LOCAL_FILE"
    chmod 600 "$ENV_LOCAL_FILE"

    log "      wrote $ENV_LOCAL_FILE (chmod 600, DB key decodes to 32 bytes)"
    log ""
    log "      Source it manually with:"
    log "          source $ENV_LOCAL_FILE && make restart-live-gateway"
    log "      Or just re-run 'make restart-live-gateway' — it auto-sources."

    # shellcheck disable=SC1090
    source "$ENV_LOCAL_FILE"
}

if [[ -f "$ENV_LOCAL_FILE" ]]; then
    log "[0/8] Auto-sourcing $ENV_LOCAL_FILE..."
    # shellcheck disable=SC1090
    source "$ENV_LOCAL_FILE"
elif [[ -z "${SKYCOT_DB_KEY_BASE64:-}" ]]; then
    log "[0/8] First-run setup: no $ENV_LOCAL_FILE and SKYCOT_DB_KEY_BASE64 unset."
    bootstrap_env_local
else
    log "[0/8] No $ENV_LOCAL_FILE but SKYCOT_DB_KEY_BASE64 is set in env; continuing."
fi

if [[ "$BOOTSTRAP_ONLY" -eq 1 ]]; then
    log "✅ --bootstrap-only requested; exiting without launching the gateway."
    exit 0
fi

# ---------- expected schema version (from source of truth) -----------

EXPECTED_SCHEMA_VERSION="$(
    grep -E '^pub const EXPECTED_SCHEMA_VERSION: u32' \
        crates/argo-store/src/lib.rs \
        | sed -E 's/.*= *([0-9]+).*/\1/'
)"
[[ -n "$EXPECTED_SCHEMA_VERSION" ]] \
    || die "could not parse EXPECTED_SCHEMA_VERSION from crates/argo-store/src/lib.rs"

# ---------- step 1: capture old PID + pre-version --------------------

log "[1/8] Capturing old gateway PID + pre-restart schema_version..."

OLD_PID=""
OLD_VER="stopped"

# Prefer a live-port probe; fall back to pgrep.
if PID_FROM_PORT=$(lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t 2>/dev/null | head -n1); then
    OLD_PID="$PID_FROM_PORT"
fi
if [[ -z "$OLD_PID" ]]; then
    if PID_FROM_PGREP=$(pgrep -f 'argo gateway start' 2>/dev/null | head -n1); then
        OLD_PID="$PID_FROM_PGREP"
    fi
fi

if [[ -n "$OLD_PID" ]]; then
    log "      found gateway pid=$OLD_PID listening on $HOST:$PORT"
    if RESP=$(curl -fsS --max-time 3 "$HEALTH_URL" 2>/dev/null); then
        # /readyz returns JSON-ish; cheap parse without jq dependency
        OLD_VER="$(printf '%s' "$RESP" \
            | sed -E 's/.*"schema_version" *: *([0-9]+).*/\1/')"
        [[ "$OLD_VER" =~ ^[0-9]+$ ]] || OLD_VER="unknown"
    else
        OLD_VER="unhealthy"
    fi
    log "      pre-restart schema_version=$OLD_VER (expected $EXPECTED_SCHEMA_VERSION)"
else
    log "      no gateway running on $HOST:$PORT"
fi

# ---------- step 2: short-circuit if current -------------------------

log "[2/8] Comparing live schema_version against EXPECTED_SCHEMA_VERSION..."

if [[ -n "$OLD_PID" && "$OLD_VER" == "$EXPECTED_SCHEMA_VERSION" ]]; then
    if [[ "$FORCE" -eq 1 ]]; then
        log "      already at $EXPECTED_SCHEMA_VERSION but --force given: continuing restart."
    else
        log "✅ already at schema_version $OLD_VER (current). No-op."
        log "      (use --force to restart anyway, e.g. to pick up a new env.local key.)"
        exit 0
    fi
fi

# ---------- step 3: pre-restart prompt -------------------------------

if [[ "$NO_PROMPT" -eq 0 ]]; then
    log "[3/8] May briefly interrupt active integration tests."
    log "      Press Ctrl-C within 5s to abort..."
    sleep 5
else
    log "[3/8] --no-prompt: skipping 5s warning."
fi

# ---------- step 4: stop old gateway ---------------------------------

if [[ -n "$OLD_PID" ]]; then
    log "[4/8] SIGTERM pid=$OLD_PID (10s grace)..."
    if ! kill -TERM "$OLD_PID" 2>/dev/null; then
        log "      kill -TERM failed (process already gone?)"
    fi
    for i in $(seq 1 10); do
        if ! kill -0 "$OLD_PID" 2>/dev/null; then
            log "      pid $OLD_PID exited after ${i}s"
            break
        fi
        sleep 1
    done
    if kill -0 "$OLD_PID" 2>/dev/null; then
        log "      pid $OLD_PID still alive after 10s — escalating to SIGKILL"
        kill -KILL "$OLD_PID" 2>/dev/null || true
        sleep 1
        if kill -0 "$OLD_PID" 2>/dev/null; then
            die "pid $OLD_PID survived SIGKILL; refusing to launch a second gateway"
        fi
    fi
else
    log "[4/8] No old gateway to stop."
fi

# ---------- step 5: rebuild-if-stale ---------------------------------

log "[5/8] Checking whether the release binary needs a rebuild..."

CUR_HEAD="$(git rev-parse HEAD)"
NEEDS_REBUILD=0

if [[ ! -x "$BIN" ]]; then
    log "      $BIN missing — will build."
    NEEDS_REBUILD=1
elif [[ ! -f "$BUILT_FROM_FILE" ]]; then
    log "      no built-from sidecar ($BUILT_FROM_FILE) — will build."
    NEEDS_REBUILD=1
elif [[ "$(cat "$BUILT_FROM_FILE")" != "$CUR_HEAD" ]]; then
    log "      built-from $(cat "$BUILT_FROM_FILE") != HEAD $CUR_HEAD — will build."
    NEEDS_REBUILD=1
else
    log "      binary up to date (built from $CUR_HEAD)."
fi

if [[ "$NEEDS_REBUILD" -eq 1 ]]; then
    log "      running: cargo build --release --bin argo"
    if ! cargo build --release --bin argo; then
        die "cargo build --release --bin argo failed; aborting restart"
    fi
    mkdir -p "$(dirname "$BUILT_FROM_FILE")"
    printf '%s\n' "$CUR_HEAD" > "$BUILT_FROM_FILE"
fi

[[ -x "$BIN" ]] || die "expected binary $BIN missing after build"

# ---------- step 6: launch new gateway detached ----------------------

log "[6/8] Launching $BIN gateway start --bind $HOST --port $PORT (detached)..."

mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/gateway-$(date +%Y%m%dT%H%M%S).log"

nohup "$BIN" gateway start --bind "$HOST" --port "$PORT" \
    >"$LOG_FILE" 2>&1 < /dev/null &
NEW_PID=$!
disown 2>/dev/null || true
log "      new pid=$NEW_PID, log=$LOG_FILE"

# ---------- step 7: poll /readyz -------------------------------------

log "[7/8] Polling $HEALTH_URL up to 30s..."

NEW_VER=""
for i in $(seq 1 30); do
    if ! kill -0 "$NEW_PID" 2>/dev/null; then
        die "gateway pid $NEW_PID exited during boot; see $LOG_FILE"
    fi
    if RESP=$(curl -fsS --max-time 2 "$HEALTH_URL" 2>/dev/null); then
        NEW_VER="$(printf '%s' "$RESP" \
            | sed -E 's/.*"schema_version" *: *([0-9]+).*/\1/')"
        if [[ "$NEW_VER" =~ ^[0-9]+$ ]]; then
            log "      /readyz returned 200 after ${i}s"
            break
        fi
    fi
    sleep 1
done

if [[ -z "$NEW_VER" || ! "$NEW_VER" =~ ^[0-9]+$ ]]; then
    die "gateway did not respond healthy on $HEALTH_URL within 30s; see $LOG_FILE"
fi

# ---------- step 8: confirm + diff -----------------------------------

log "[8/8] Confirming new schema_version >= EXPECTED_SCHEMA_VERSION..."

if (( NEW_VER < EXPECTED_SCHEMA_VERSION )); then
    die "post-restart schema_version=$NEW_VER < expected=$EXPECTED_SCHEMA_VERSION; \
likely a stale binary. Run: rm -f $BUILT_FROM_FILE && $0"
fi

log "✅ Gateway live at schema_version $NEW_VER (was $OLD_VER)."
