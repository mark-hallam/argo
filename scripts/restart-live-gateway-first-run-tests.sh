#!/usr/bin/env bash
# Smoke tests for the first-run bootstrap path in
# scripts/restart-live-gateway.sh.
#
# Scenarios covered:
#   (1) First run on a clean $ARGO_HOME → bootstrap writes env.local
#       with mode 0600, SKYCOT_DB_KEY_BASE64 decodes to exactly 32
#       bytes, ANTHROPIC_API_KEY from the parent env is copied in.
#   (2) Re-run with env.local present → bootstrap is skipped, the
#       script auto-sources the existing file without rewriting it.
#   (3) Bootstrap with an unusually-shaped API key → warning is
#       printed but the value is still written (per acceptance
#       criterion: warn but accept).
#   (4) Non-interactive shell with no *_API_KEY in env → env.local
#       still gets written (placeholder API line) so the operator
#       can edit it in.
#
# These tests invoke `restart-live-gateway.sh --bootstrap-only` so
# they never touch a real gateway / cargo build. The whole suite runs
# in a temp $ARGO_HOME to keep the operator's actual ~/.argo
# untouched.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"
[[ -n "$REPO_ROOT" ]] || { echo "must run inside argo git repo" >&2; exit 1; }
cd "$REPO_ROOT"

SCRIPT="$REPO_ROOT/scripts/restart-live-gateway.sh"
[[ -x "$SCRIPT" ]] || { echo "script not executable: $SCRIPT" >&2; exit 1; }

TMPDIR="$(mktemp -d -t restart-bootstrap-smoke.XXXXXX)"
trap 'rm -rf "$TMPDIR"' EXIT

pass() { printf '  ✅ %s\n' "$*"; }
fail() { printf '  ❌ %s\n' "$*" >&2; exit 1; }

# Each scenario runs in its own fake ARGO_HOME under TMPDIR. We
# inherit CARGO_TARGET_DIR from the caller (or fall back to a tmp
# path) so the script doesn't probe ./target. ARGO_RESTART_BIN
# points at /bin/true so the bootstrap-only path never tries to
# exec a missing binary.
run_bootstrap() {
    local home="$1"; shift
    mkdir -p "$home"
    env -i \
        HOME="$home" \
        PATH="/usr/local/bin:/usr/bin:/bin" \
        ARGO_HOME="$home/.argo" \
        ARGO_RESTART_BIN="/bin/true" \
        CARGO_TARGET_DIR="$TMPDIR/cargo-target" \
        ARGO_BOOTSTRAP_NONINTERACTIVE=1 \
        "$@" \
        bash "$SCRIPT" --bootstrap-only --no-prompt
}

decoded_length() {
    printf '%s' "$1" | base64 -d 2>/dev/null | wc -c | tr -d ' '
}

# ---------- scenario 1: first run + parent ANTHROPIC_API_KEY ----------

echo "Scenario 1: clean ARGO_HOME with ANTHROPIC_API_KEY in parent env."
H1="$TMPDIR/home1"
if out=$(run_bootstrap "$H1" ANTHROPIC_API_KEY='sk-ant-test-key-pretend-real-12345' 2>&1); then
    pass "exit 0"
else
    echo "$out" >&2
    fail "bootstrap-only exited non-zero on first run"
fi
ENV_FILE="$H1/.argo/env.local"
[[ -f "$ENV_FILE" ]] || fail "expected $ENV_FILE to exist"
PERMS="$(stat -f '%Lp' "$ENV_FILE" 2>/dev/null || stat -c '%a' "$ENV_FILE")"
[[ "$PERMS" == "600" ]] || fail "expected mode 600 on env.local, got $PERMS"
pass "env.local exists with mode 600"

set +u
# shellcheck disable=SC1090
source "$ENV_FILE"
set -u
LEN=$(decoded_length "$SKYCOT_DB_KEY_BASE64")
[[ "$LEN" == "32" ]] || fail "SKYCOT_DB_KEY_BASE64 decodes to $LEN bytes, want 32"
pass "SKYCOT_DB_KEY_BASE64 decodes to 32 bytes"
[[ "$ANTHROPIC_API_KEY" == "sk-ant-test-key-pretend-real-12345" ]] \
    || fail "ANTHROPIC_API_KEY did not survive the round-trip"
pass "ANTHROPIC_API_KEY copied from parent env"
echo "$out" | grep -q "reusing" \
    || fail "expected 'reusing' message when parent env had ANTHROPIC_API_KEY"
pass "bootstrap detected and reused parent ANTHROPIC_API_KEY"
unset SKYCOT_DB_KEY_BASE64 ANTHROPIC_API_KEY

# ---------- scenario 2: re-run is idempotent (auto-source only) -------

echo "Scenario 2: re-run with env.local present → no rewrite, auto-source only."
SIG_BEFORE="$(shasum "$ENV_FILE" | awk '{print $1}')"
MTIME_BEFORE="$(stat -f '%m' "$ENV_FILE" 2>/dev/null || stat -c '%Y' "$ENV_FILE")"
sleep 1
if out=$(run_bootstrap "$H1" 2>&1); then
    pass "exit 0 on re-run"
else
    echo "$out" >&2
    fail "bootstrap-only exited non-zero on re-run"
fi
SIG_AFTER="$(shasum "$ENV_FILE" | awk '{print $1}')"
MTIME_AFTER="$(stat -f '%m' "$ENV_FILE" 2>/dev/null || stat -c '%Y' "$ENV_FILE")"
[[ "$SIG_BEFORE" == "$SIG_AFTER" ]] \
    || fail "env.local contents changed on re-run (sha mismatch)"
[[ "$MTIME_BEFORE" == "$MTIME_AFTER" ]] \
    || fail "env.local mtime changed on re-run (rewritten)"
pass "env.local untouched on re-run"
echo "$out" | grep -q "Auto-sourcing" \
    || fail "expected 'Auto-sourcing' message on re-run"
pass "auto-source path took effect"

# ---------- scenario 3: unusual API key → warn but accept ------------

echo "Scenario 3: unusual API key shape → warning printed, value still written."
H3="$TMPDIR/home3"
if out=$(run_bootstrap "$H3" ANTHROPIC_API_KEY='shortkey' 2>&1); then
    pass "exit 0"
else
    echo "$out" >&2
    fail "bootstrap-only exited non-zero with unusual API key"
fi
echo "$out" | grep -qi "warning: API key looks unusual" \
    || fail "expected unusual-key warning in output"
pass "warning printed"
# shellcheck disable=SC1090
set +u
# shellcheck disable=SC1090
source "$H3/.argo/env.local"
set -u
[[ "$ANTHROPIC_API_KEY" == "shortkey" ]] \
    || fail "unusual API key was not written through despite warning"
pass "unusual API key written through despite warning"
unset SKYCOT_DB_KEY_BASE64 ANTHROPIC_API_KEY

# ---------- scenario 4: non-interactive + no *_API_KEY ---------------

echo "Scenario 4: non-interactive with no *_API_KEY → env.local has placeholder."
H4="$TMPDIR/home4"
if out=$(run_bootstrap "$H4" 2>&1); then
    pass "exit 0"
else
    echo "$out" >&2
    fail "bootstrap-only exited non-zero with no API key"
fi
ENV_FILE4="$H4/.argo/env.local"
[[ -f "$ENV_FILE4" ]] || fail "expected $ENV_FILE4 to exist"
grep -q "SKYCOT_DB_KEY_BASE64=" "$ENV_FILE4" \
    || fail "env.local missing SKYCOT_DB_KEY_BASE64 line"
grep -q "# export ANTHROPIC_API_KEY" "$ENV_FILE4" \
    || fail "env.local missing commented placeholder ANTHROPIC_API_KEY"
pass "env.local has DB key + commented API-key placeholder"

echo "✅ All 4 first-run bootstrap scenarios passed."
