#!/usr/bin/env bash
# Hermetic smoke harness for scripts/check-ledger-not-eaten-by-stash-pop.sh.
#
# Sets up a throw-away git repo under mktemp -d, plants a fake
# BUILD_STATUS.md, commits it, then exercises four scenarios:
#   (A) TODO row deleted + not relocated         → expect exit 1
#   (B) TODO row deleted but re-added elsewhere  → expect exit 0
#   (C) MERGED row deleted                       → expect exit 0
#   (D) no diff to the ledger                    → expect exit 0
# A 5th sanity case exercises the --warn flag against scenario A → exit 0.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT="$SCRIPT_DIR/check-ledger-not-eaten-by-stash-pop.sh"
[[ -x "$SCRIPT" ]] || { echo "script not executable: $SCRIPT" >&2; exit 1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cd "$TMP"
git init -q -b main
git config user.email "smoke@test.invalid"
git config user.name "Smoke Test"
mkdir -p docs/build-plan

write_baseline() {
    cat >docs/build-plan/BUILD_STATUS.md <<'LEDGER'
# Build status

| Brief | Owner | Status | PR | Last update |
| --- | --- | --- | --- | --- |
| **▼ PHASE A — top priority** | — | — | — | — |
| alpha-brief-todo | claude | TODO | — | claimed via retry |
| beta-brief-claimed | codex | CLAIMED | — | working on impl |
| gamma-brief-merged | claude | MERGED | #1234 | shipped 2026-05-25 |
| delta-brief-blocked | codex | BLOCKED | — | premise stale |
LEDGER
    git add docs/build-plan/BUILD_STATUS.md
    git commit -q -m baseline
}

run_check() {
    set +e
    if (( $# == 0 )); then
        OUT="$("$SCRIPT" 2>&1)"
    else
        OUT="$("$SCRIPT" "$@" 2>&1)"
    fi
    RC=$?
    set -e
}

pass() { printf '  ✅ %s\n' "$*"; }
fail() { printf '  ❌ %s\n' "$*" >&2; printf '%s\n' "$OUT" | sed 's/^/      /' >&2; exit 1; }

write_baseline

# --- A: TODO row deleted, not relocated -> exit 1 ---
echo "Scenario A: TODO row deleted (no relocation) → exit 1."
sed -i.bak '/alpha-brief-todo/d' docs/build-plan/BUILD_STATUS.md
rm -f docs/build-plan/BUILD_STATUS.md.bak
run_check
[[ $RC -eq 1 ]] || fail "A: expected exit 1, got $RC"
printf '%s\n' "$OUT" | grep -q "alpha-brief-todo" || fail "A: dropped brief-id not surfaced"
pass "A exit 1 + brief-id surfaced"

# --warn against the same state -> exit 0
echo "Scenario A2: --warn downgrades to exit 0."
run_check --warn
[[ $RC -eq 0 ]] || fail "A2: expected exit 0 with --warn, got $RC"
printf '%s\n' "$OUT" | grep -q "warn-only" || fail "A2: missing warn-only banner"
pass "A2 --warn exit 0 + banner"

# reset to baseline for next case
git checkout -q -- docs/build-plan/BUILD_STATUS.md

# --- B: TODO row deleted, brief-id re-added elsewhere -> exit 0 ---
echo "Scenario B: TODO row deleted but brief-id reappears elsewhere → exit 0."
sed -i.bak '/alpha-brief-todo/d' docs/build-plan/BUILD_STATUS.md
rm -f docs/build-plan/BUILD_STATUS.md.bak
# Re-add the brief-id at end of file with same status.
printf '| alpha-brief-todo | claude | TODO | — | re-ordered |\n' \
    >>docs/build-plan/BUILD_STATUS.md
run_check
[[ $RC -eq 0 ]] || fail "B: expected exit 0 (relocation), got $RC; out: $OUT"
pass "B exit 0 (relocation accepted)"
git checkout -q -- docs/build-plan/BUILD_STATUS.md

# --- C: MERGED row deleted -> exit 0 ---
echo "Scenario C: MERGED row deleted → exit 0 (only TODO/CLAIMED is guarded)."
sed -i.bak '/gamma-brief-merged/d' docs/build-plan/BUILD_STATUS.md
rm -f docs/build-plan/BUILD_STATUS.md.bak
run_check
[[ $RC -eq 0 ]] || fail "C: expected exit 0 (MERGED loss is ignored), got $RC; out: $OUT"
pass "C exit 0 (MERGED row drop ignored)"
git checkout -q -- docs/build-plan/BUILD_STATUS.md

# --- D: no diff -> exit 0 ---
echo "Scenario D: no diff to the ledger → exit 0."
run_check
[[ $RC -eq 0 ]] || fail "D: expected exit 0 (no diff), got $RC; out: $OUT"
pass "D exit 0 (clean working tree)"

# --- E: --staged variant exits 1 once row deletion is staged ---
echo "Scenario E: --staged variant detects the same loss when staged."
sed -i.bak '/beta-brief-claimed/d' docs/build-plan/BUILD_STATUS.md
rm -f docs/build-plan/BUILD_STATUS.md.bak
git add docs/build-plan/BUILD_STATUS.md
run_check --staged
[[ $RC -eq 1 ]] || fail "E: expected exit 1 from --staged, got $RC; out: $OUT"
printf '%s\n' "$OUT" | grep -q "beta-brief-claimed" || fail "E: CLAIMED brief-id not surfaced"
pass "E --staged exit 1 + CLAIMED brief-id surfaced"

echo ""
echo "STATUS: GREEN — 5/5 scenarios PASS"
