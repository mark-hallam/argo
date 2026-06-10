#!/usr/bin/env bash
# conductor-pre-merge-gate-tests.sh — smoke harness for the pre-merge
# gate's new test-compile step (`scripts/pre-merge-test-compile.sh`,
# gate 6 of `scripts/conductor-pre-merge-gate.sh`).
#
# Covers four cases:
#   (a) clean repo                      → gate PASSES
#   (b) unallowlisted broken test       → gate FAILS
#   (c) allowlisted broken test         → gate PASS-WITH-WARNING
#   (d) parallel-pub-types --enforce    → synthetic cross-crate duplicate
#                                         pub type FAILS, allowlisting clears it
#
# Mutates these paths and restores them via EXIT trap regardless of
# outcome:
#   - injects a temp broken test at $TARGET_CRATE/tests/$BROKEN_REL
#   - appends a path entry to scripts/pre-merge-test-carry-allowlist.txt
#   - injects two synthetic dup-type files (case d) + a temp allowlist entry
#
# Usage:
#   bash scripts/conductor-pre-merge-gate-tests.sh
#
# Exit codes:
#   0 — all three cases behaved as expected
#   1 — at least one case misbehaved (gate broken or environment red)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

GATE_SCRIPT="scripts/pre-merge-test-compile.sh"
ALLOWLIST_FILE="scripts/pre-merge-test-carry-allowlist.txt"

# Inject the broken test into a small crate with an existing tests/
# folder. argo-core fits: it's a leaf-ish crate (small dep graph) and
# already has tests/surface_contract.rs, so a sibling test file does
# not introduce new build infrastructure.
TARGET_CRATE="crates/argo-core"
BROKEN_REL="tests/__smoke_harness_broken__.rs"
BROKEN_FILE="$TARGET_CRATE/$BROKEN_REL"

PASS=0
FAIL=0
ALLOWLIST_BACKUP=""

# Case (d) — synthetic parallel-pub-types fixtures. The linter globs
# crates/*/src/**/*.rs by file text alone (no module wiring needed), so
# two unreferenced files in two different crates declaring the same
# `pub struct` are enough to trip a cross-crate duplicate.
PUBTYPES_LINT="scripts/lint-parallel-pub-types.py"
PUBTYPES_ALLOWLIST="scripts/lint-parallel-pub-types.allowlist"
SYNTH_NAME="__SyntheticGateDup__"
SYNTH_A="crates/argo-core/src/__synthetic_gate_dup__.rs"
SYNTH_B="crates/argo-types/src/__synthetic_gate_dup__.rs"
PUBTYPES_ALLOWLIST_BACKUP=""

cleanup() {
  [[ -f "$BROKEN_FILE" ]] && rm -f "$BROKEN_FILE"
  if [[ -n "$ALLOWLIST_BACKUP" && -f "$ALLOWLIST_BACKUP" ]]; then
    cp "$ALLOWLIST_BACKUP" "$ALLOWLIST_FILE"
    rm -f "$ALLOWLIST_BACKUP"
  fi
  [[ -f "$SYNTH_A" ]] && rm -f "$SYNTH_A"
  [[ -f "$SYNTH_B" ]] && rm -f "$SYNTH_B"
  if [[ -n "$PUBTYPES_ALLOWLIST_BACKUP" && -f "$PUBTYPES_ALLOWLIST_BACKUP" ]]; then
    cp "$PUBTYPES_ALLOWLIST_BACKUP" "$PUBTYPES_ALLOWLIST"
    rm -f "$PUBTYPES_ALLOWLIST_BACKUP"
  fi
}
trap cleanup EXIT

note() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  PASS  %s\n' "$*"; PASS=$((PASS + 1)); }
bad()  { printf '  FAIL  %s\n' "$*"; FAIL=$((FAIL + 1)); }

ALLOWLIST_BACKUP=$(mktemp)
cp "$ALLOWLIST_FILE" "$ALLOWLIST_BACKUP"

# ---------------------------------------------------------------------
# (a) clean repo — gate exits 0
# ---------------------------------------------------------------------
note "Case (a): clean repo passes"
if bash "$GATE_SCRIPT" --quiet; then
  ok "gate exits 0 on clean repo"
else
  bad "gate failed on clean repo — main may be red, or gate is broken"
fi

# ---------------------------------------------------------------------
# (b) broken test, not allowlisted — gate exits 1
# ---------------------------------------------------------------------
note "Case (b): unallowlisted broken test fails"
cat > "$BROKEN_FILE" <<'RUST'
//! smoke-harness fixture — intentionally broken; deleted on cleanup.
#[test]
fn intentionally_broken() {
    let _: i32 = "this is not an i32";
}
RUST
out_b=$(bash "$GATE_SCRIPT" --quiet 2>&1)
rc_b=$?
if [[ "$rc_b" -ne 0 ]]; then
  ok "gate exits non-zero on unallowlisted broken test (rc=$rc_b)"
else
  bad "gate exited 0 with broken test (should fail). Output:"
  echo "$out_b" | tail -10
fi

# ---------------------------------------------------------------------
# (c) broken test, allowlisted — gate exits 0 with PASS-WITH-WARNING
# ---------------------------------------------------------------------
note "Case (c): allowlisted broken test passes with warning"
# Append the broken-file path; substring match in the gate will catch it.
echo "$BROKEN_REL" >> "$ALLOWLIST_FILE"
out_c=$(bash "$GATE_SCRIPT" 2>&1)
rc_c=$?
if [[ "$rc_c" -eq 0 ]] && echo "$out_c" | grep -q "PASS-WITH-WARNING"; then
  ok "gate exits 0 with PASS-WITH-WARNING for allowlisted broken test"
else
  bad "gate did not pass-with-warning (rc=$rc_c). Output:"
  echo "$out_c" | tail -15
fi

# ---------------------------------------------------------------------
# (d) parallel-pub-types --enforce — synthetic cross-crate duplicate
#     pub type FAILS; an allowlist entry then clears it.
# ---------------------------------------------------------------------
note "Case (d): parallel-pub-types --enforce fails on a synthetic duplicate"
printf 'pub struct %s;\n' "$SYNTH_NAME" > "$SYNTH_A"
printf 'pub struct %s;\n' "$SYNTH_NAME" > "$SYNTH_B"

# Sanity: warn-only mode (no --enforce) still exits 0 even with the dup.
python3 "$PUBTYPES_LINT" >/dev/null 2>&1
rc_warn=$?
if [[ "$rc_warn" -eq 0 ]]; then
  ok "linter without --enforce stays exit 0 (warn-only) despite the dup"
else
  bad "linter without --enforce exited $rc_warn (expected 0)"
fi

# --enforce must reject the duplicate.
out_d=$(python3 "$PUBTYPES_LINT" --enforce 2>&1)
rc_d=$?
if [[ "$rc_d" -ne 0 ]] && echo "$out_d" | grep -q "$SYNTH_NAME"; then
  ok "linter --enforce exits non-zero on synthetic duplicate (rc=$rc_d)"
else
  bad "linter --enforce did not fail on the duplicate (rc=$rc_d). Output:"
  echo "$out_d" | tail -10
fi

# Allowlisting the pair clears the finding.
PUBTYPES_ALLOWLIST_BACKUP=$(mktemp)
cp "$PUBTYPES_ALLOWLIST" "$PUBTYPES_ALLOWLIST_BACKUP"
echo "$SYNTH_NAME argo-core argo-types  # smoke-harness synthetic fixture" >> "$PUBTYPES_ALLOWLIST"
python3 "$PUBTYPES_LINT" --enforce >/dev/null 2>&1
rc_allow=$?
if [[ "$rc_allow" -eq 0 ]]; then
  ok "linter --enforce exits 0 once the synthetic pair is allowlisted"
else
  bad "linter --enforce stayed non-zero after allowlisting (rc=$rc_allow)"
fi

echo
echo "smoke harness: $PASS pass / $FAIL fail"
[[ "$FAIL" -gt 0 ]] && exit 1
exit 0
