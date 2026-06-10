#!/usr/bin/env bash
# Hermetic tests for scripts/codesign-gateway-binary.sh.
#
# The real macOS `codesign` / `security` tools are only exercised in the
# stubbed success scenario (D) — the skip/fallback scenarios use the real
# tools but never mutate the keychain. No scenario touches the operator's
# keychain or the real gateway binary.
#
# Scenarios:
#   A. no binary arg            → exit 2 (misuse).
#   B. ARGO_CODESIGN_IDENTITY unset → exit 0, "leaving ad-hoc signature".
#   C. identity set but absent  → exit 0, "not found in any keychain".
#   D. stubbed identity present → exit 0, signs with --identifier, reports
#                                 "stable signature applied".

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"
[[ -n "$REPO_ROOT" ]] || { echo "must run inside argo git repo" >&2; exit 1; }
cd "$REPO_ROOT"

SCRIPT="$REPO_ROOT/scripts/codesign-gateway-binary.sh"
[[ -x "$SCRIPT" ]] || { echo "script not executable: $SCRIPT" >&2; exit 1; }

TMPDIR="$(mktemp -d -t codesign-gateway-binary.XXXXXX)"
trap 'rm -rf "$TMPDIR"' EXIT

FAKE_BIN="$TMPDIR/argo"
printf '#!/bin/sh\nexit 0\n' > "$FAKE_BIN"
chmod +x "$FAKE_BIN"

pass() { printf '  ✅ %s\n' "$*"; }
fail() { printf '  ❌ %s\n' "$*" >&2; exit 1; }

# ---------- scenario A: missing arg ----------------------------------

echo "Scenario A: no binary arg → exit 2."
if out=$(unset ARGO_CODESIGN_IDENTITY; "$SCRIPT" 2>&1); then
    echo "$out" >&2; fail "expected non-zero exit with no arg"
else
    rc=$?
    [[ "$rc" -eq 2 ]] || { echo "$out" >&2; fail "expected exit 2, got $rc"; }
    pass "exit 2 on missing arg"
fi

# Scenarios below only assert behaviour meaningfully on macOS, where the
# real codesign/security exist. On non-Darwin the script short-circuits at
# the uname guard; assert that path instead.
if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "Non-macOS host: asserting uname short-circuit only."
    out=$(ARGO_CODESIGN_IDENTITY=whatever "$SCRIPT" "$FAKE_BIN" 2>&1) \
        || fail "expected exit 0 on non-macOS"
    echo "$out" | grep -q "not macOS" || { echo "$out" >&2; fail "missing 'not macOS' line"; }
    pass "non-macOS short-circuit"
    echo "✅ codesign-gateway-binary tests passed (non-macOS subset)."
    exit 0
fi

# ---------- scenario B: identity unset → skip ------------------------

echo "Scenario B: ARGO_CODESIGN_IDENTITY unset → leave ad-hoc."
out=$(unset ARGO_CODESIGN_IDENTITY; "$SCRIPT" "$FAKE_BIN" 2>&1) \
    || fail "expected exit 0 when identity unset"
echo "$out" | grep -q "leaving ad-hoc signature" \
    || { echo "$out" >&2; fail "missing 'leaving ad-hoc signature' line"; }
pass "skips cleanly with no identity configured"

# ---------- scenario C: identity set but not in keychain -------------

echo "Scenario C: identity set but absent from keychain → leave ad-hoc."
BOGUS="skycot-nonexistent-identity-$$-do-not-create"
out=$(ARGO_CODESIGN_IDENTITY="$BOGUS" "$SCRIPT" "$FAKE_BIN" 2>&1) \
    || fail "expected exit 0 when identity absent"
echo "$out" | grep -q "not found in any keychain" \
    || { echo "$out" >&2; fail "missing 'not found in any keychain' line"; }
pass "skips cleanly when identity absent"

# ---------- scenario D: stubbed identity present → signs -------------

echo "Scenario D: stubbed identity present → signs with --identifier."
STUBDIR="$TMPDIR/stubbin"
mkdir -p "$STUBDIR"
CODESIGN_LOG="$TMPDIR/codesign-args.log"
STUB_IDENTITY="skycot-stub-identity"

# Stub `security`: report the stub identity for the codesigning query.
cat > "$STUBDIR/security" <<EOF
#!/usr/bin/env bash
if [[ "\$1" == "find-identity" ]]; then
    printf '  1) DEADBEEFDEADBEEF "%s"\n' "$STUB_IDENTITY"
    printf '     1 valid identities found\n'
    exit 0
fi
exit 0
EOF
chmod +x "$STUBDIR/security"

# Stub `codesign`: record the sign invocation; report a non-adhoc identity
# for the `-dv` verification query.
cat > "$STUBDIR/codesign" <<EOF
#!/usr/bin/env bash
if [[ "\$1" == "-dv" || "\$1" == "-d" ]]; then
    echo "Identifier=com.skycot.argo" >&2
    echo "Signature=stable" >&2
    exit 0
fi
printf '%s\n' "\$*" >> "$CODESIGN_LOG"
exit 0
EOF
chmod +x "$STUBDIR/codesign"

out=$(PATH="$STUBDIR:$PATH" ARGO_CODESIGN_IDENTITY="$STUB_IDENTITY" \
        "$SCRIPT" "$FAKE_BIN" 2>&1) \
    || { echo "$out" >&2; fail "expected exit 0 in stubbed success path"; }
echo "$out" | grep -q "stable signature applied" \
    || { echo "$out" >&2; fail "missing 'stable signature applied' line"; }
[[ -f "$CODESIGN_LOG" ]] || fail "codesign sign was never invoked"
grep -q -- "--identifier com.skycot.argo" "$CODESIGN_LOG" \
    || { cat "$CODESIGN_LOG" >&2; fail "sign invocation missing '--identifier com.skycot.argo'"; }
grep -qF -- "--sign $STUB_IDENTITY" "$CODESIGN_LOG" \
    || { cat "$CODESIGN_LOG" >&2; fail "sign invocation missing '--sign $STUB_IDENTITY'"; }
pass "signs with --sign $STUB_IDENTITY --identifier com.skycot.argo"

# Custom identifier override is honoured.
: > "$CODESIGN_LOG"
out=$(PATH="$STUBDIR:$PATH" ARGO_CODESIGN_IDENTITY="$STUB_IDENTITY" \
        ARGO_CODESIGN_IDENTIFIER="com.example.custom" \
        "$SCRIPT" "$FAKE_BIN" 2>&1) \
    || { echo "$out" >&2; fail "expected exit 0 with custom identifier"; }
grep -q -- "--identifier com.example.custom" "$CODESIGN_LOG" \
    || { cat "$CODESIGN_LOG" >&2; fail "custom ARGO_CODESIGN_IDENTIFIER not honoured"; }
pass "ARGO_CODESIGN_IDENTIFIER override honoured"

echo "✅ All codesign-gateway-binary scenarios passed."
