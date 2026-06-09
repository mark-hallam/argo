#!/usr/bin/env bash
# test-capture-recurring-nits.sh — deterministic verifier for the DEC-042 B-3
# recurring-nit capturer (scripts/capture-recurring-nits.py).
#
# No network, no git dependency (--no-git), fixture ledger only. Run it twice
# to confirm determinism. Asserts:
#   1. threshold honored — a phrase repeated 4x is emitted, a 1x phrase is not.
#   2. --draft writes exactly ONE capture-*.md with a valid freshness header.
#   3. idempotence — re-running --draft writes no new draft and no longer emits
#      the now-captured pattern.
#
# Usage: bash scripts/test-capture-recurring-nits.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CAPTURE="$REPO_ROOT/scripts/capture-recurring-nits.py"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

LEDGER="$TMP/BUILD_STATUS.md"
STREAMS="$TMP/streams"
KNOWN="$TMP/KNOWN_FLAKES.md"
mkdir -p "$STREAMS"
: > "$KNOWN"

PASS=0
FAIL=0
check() { # check <desc> <condition-exit-code>
  if [[ "$2" -eq 0 ]]; then echo "  PASS: $1"; PASS=$((PASS+1));
  else echo "  FAIL: $1"; FAIL=$((FAIL+1)); fi
}

# Fixture: the token `frobnicate_widget` appears in a nit context 4x; the token
# `solitary_oneoff_nit` appears once. Both carry a "pre-existing" nit marker.
cat > "$LEDGER" <<'EOF'
| brief-id | owner | state | pr | note |
|---|---|---|---|---|
| brief-a | claude | MERGED | #1 | shipped feature A. Boundary: pre-existing frobnicate_widget clippy blocker, outside diff. |
| brief-b | claude | MERGED | #2 | shipped feature B. pre-existing frobnicate_widget clippy error remains. |
| brief-c | codex | MERGED | #3 | feature C. boundary note: pre-existing frobnicate_widget flake in untouched file. |
| brief-d | claude | MERGED | #4 | feature D done. known flake: frobnicate_widget pre-existing, boundary-noted. |
| brief-e | claude | MERGED | #5 | feature E. pre-existing solitary_oneoff_nit clippy thing, outside may-touch. |
EOF

# --- Test 1: threshold honored -------------------------------------------------
OUT="$(python3 "$CAPTURE" --no-git --ledger "$LEDGER" --streams-dir "$STREAMS" \
  --known-flakes "$KNOWN" --threshold 3 2>/dev/null)"

echo "$OUT" | python3 -c '
import json,sys
d=json.load(sys.stdin)
pats={f["pattern"] for f in d["findings"]}
assert "frobnicate_widget" in pats, f"4x phrase missing: {pats}"
assert "solitary_oneoff_nit" not in pats, f"1x phrase leaked: {pats}"
fw=[f for f in d["findings"] if f["pattern"]=="frobnicate_widget"][0]
assert fw["count"]==4, f"expected count 4, got {fw['count']}"
'
check "threshold: 4x phrase emitted (count 4), 1x phrase suppressed" $?

# --quiet prints just the count
QC="$(python3 "$CAPTURE" --no-git --ledger "$LEDGER" --streams-dir "$STREAMS" \
  --known-flakes "$KNOWN" --threshold 3 --quiet 2>/dev/null)"
[[ "$QC" =~ ^[0-9]+$ ]]
check "--quiet emits a bare integer count (got '$QC')" $?

# --- Test 2: --draft writes exactly one valid capture brief --------------------
python3 "$CAPTURE" --no-git --ledger "$LEDGER" --streams-dir "$STREAMS" \
  --known-flakes "$KNOWN" --threshold 3 --draft >/dev/null 2>&1
DRAFTS=("$STREAMS"/capture-*.md)
[[ ${#DRAFTS[@]} -eq 1 && -f "${DRAFTS[0]}" ]]
check "--draft wrote exactly one capture-*.md" $?

if [[ -f "$STREAMS/capture-frobnicate-widget.md" ]]; then
  grep -q '^\*\*Authored against\*\*: origin/main' "$STREAMS/capture-frobnicate-widget.md" \
    && grep -q '^\*\*Validate before claim\*\*:' "$STREAMS/capture-frobnicate-widget.md"
  check "draft has a valid freshness header (Authored against + Validate before claim)" $?
  grep -qi 'DRAFT' "$STREAMS/capture-frobnicate-widget.md"
  check "draft is marked DRAFT (not queued as TODO)" $?
else
  check "draft slug is capture-frobnicate-widget.md" 1
fi

# --- Test 3: idempotence -------------------------------------------------------
OUT2="$(python3 "$CAPTURE" --no-git --ledger "$LEDGER" --streams-dir "$STREAMS" \
  --known-flakes "$KNOWN" --threshold 3 --draft 2>/dev/null)"
DRAFTS2=("$STREAMS"/capture-*.md)
[[ ${#DRAFTS2[@]} -eq 1 ]]
check "idempotence: re-running --draft writes no second draft" $?

echo "$OUT2" | python3 -c '
import json,sys
d=json.load(sys.stdin)
pats={f["pattern"] for f in d["findings"]}
written=d["drafts_written"]
assert "frobnicate_widget" not in pats, f"captured pattern re-emitted: {pats}"
assert written==[], f"unexpected drafts_written: {written}"
'
check "idempotence: captured pattern no longer emitted" $?

# --- summary -------------------------------------------------------------------
echo ""
echo "capture-recurring-nits self-test: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
