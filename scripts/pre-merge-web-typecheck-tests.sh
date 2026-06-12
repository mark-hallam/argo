#!/usr/bin/env bash
# pre-merge-web-typecheck-tests.sh — smoke harness for the web-file
# DETECTION in `scripts/pre-merge-web-typecheck.sh` (gate 7 of
# `scripts/conductor-pre-merge-gate.sh`).
#
# Origin: gate-integrity defect on PR #2860 (2026-06-12). Gate 7 printed
# "no web/** files in diff — SKIP" for a web-ONLY PR because the local
# git-diff heuristic (`origin/main...HEAD`) went degenerate in the
# conductor's checkout state, silently bypassing the only gate step that
# exercises a web PR. The fix makes GitHub's file list authoritative when
# a PR number is passed, with a loud cross-check guard for a detection miss.
#
# This harness exercises the DETECTION decision only (via the script's
# `--detect-only` mode, which prints RUN/SKIP + guard state and never runs
# tsc), so it is fast (<1s) and hermetic — `gh` is stubbed via GH_BIN and
# no node_modules / network is required. The tsc-FAILS-on-a-type-error path
# is covered by the real `pnpm tsc --noEmit` invocation in the script body
# (too slow + node-dependent to belong in this smoke harness).
#
# Cases:
#   (a) no --pr, no local web change         → SKIP
#   (b) --pr, GitHub says web, local=0       → RUN guard=TRIPPED (the #2860 miss)
#   (c) --pr, GitHub says no web             → SKIP (trust GitHub)
#   (d) --pr, GitHub says web, local web mod → RUN (no trip; signals agree)
#   (e) --pr, gh query FAILS                 → falls back to local (SKIP here)
#
# Usage:
#   bash scripts/pre-merge-web-typecheck-tests.sh
#
# Exit codes:
#   0 — every case behaved as expected
#   1 — at least one case misbehaved (detection broken or environment red)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT" || exit 1

SCRIPT="scripts/pre-merge-web-typecheck.sh"

PASS=0
FAIL=0
STUB_DIR=""
TRACKED_WEB="web/package.json"
TRACKED_WEB_BACKUP=""

cleanup() {
  if [[ -n "$TRACKED_WEB_BACKUP" && -f "$TRACKED_WEB_BACKUP" ]]; then
    cp "$TRACKED_WEB_BACKUP" "$TRACKED_WEB"
    rm -f "$TRACKED_WEB_BACKUP"
  fi
  [[ -n "$STUB_DIR" && -d "$STUB_DIR" ]] && rm -rf "$STUB_DIR"
}
trap cleanup EXIT

note() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  PASS  %s\n' "$*"; PASS=$((PASS + 1)); }
bad()  { printf '  FAIL  %s\n' "$*"; FAIL=$((FAIL + 1)); }

# Build a stub `gh` whose `pr view ... --jq '.files[].path'` prints the
# newline list in $STUB_FILES and exits $STUB_RC. Selected via GH_BIN.
STUB_DIR="$(mktemp -d)"
cat > "$STUB_DIR/gh" <<'STUB'
#!/usr/bin/env bash
# stub gh — emit canned PR file list for the smoke harness
if [[ "${STUB_RC:-0}" -ne 0 ]]; then
  exit "${STUB_RC}"
fi
printf '%s\n' "${STUB_FILES:-}"
exit 0
STUB
chmod +x "$STUB_DIR/gh"
GH_STUB="$STUB_DIR/gh"

run_detect() { GH_BIN="$GH_STUB" bash "$SCRIPT" --detect-only "$@" 2>/dev/null; }

# ---------------------------------------------------------------------
# (a) no --pr, no local web change → SKIP
# ---------------------------------------------------------------------
note "Case (a): no --pr, clean web → SKIP"
out_a="$(run_detect)"
if [[ "$out_a" == "decision=SKIP" ]]; then
  ok "no-PR clean run skips ($out_a)"
else
  bad "expected decision=SKIP, got: $out_a"
fi

# ---------------------------------------------------------------------
# (b) --pr, GitHub reports web files, local detection finds zero
#     → RUN with the cross-check guard TRIPPED (the #2860 failure mode)
# ---------------------------------------------------------------------
note "Case (b): GitHub says web, local=0 → RUN guard=TRIPPED"
out_b="$(STUB_FILES=$'web/lib/api/capability-requests.ts\nweb/components/skycot/worker/WorkerOverviewContent.tsx' run_detect --pr 2860)"
if [[ "$out_b" == "decision=RUN guard=TRIPPED" ]]; then
  ok "detection miss does NOT skip — runs + trips guard ($out_b)"
else
  bad "expected 'decision=RUN guard=TRIPPED', got: $out_b"
fi

# ---------------------------------------------------------------------
# (c) --pr, GitHub reports no web files → SKIP (trust GitHub)
# ---------------------------------------------------------------------
note "Case (c): GitHub says no web → SKIP"
out_c="$(STUB_FILES=$'crates/argo-core/src/lib.rs\ndocs/build-plan/BUILD_STATUS.md' run_detect --pr 2861)"
if [[ "$out_c" == "decision=SKIP" ]]; then
  ok "Rust/doc-only PR skips per GitHub list ($out_c)"
else
  bad "expected decision=SKIP, got: $out_c"
fi

# ---------------------------------------------------------------------
# (d) --pr, GitHub says web AND local git also sees a web change
#     → RUN, no trip (the signals agree)
# ---------------------------------------------------------------------
note "Case (d): GitHub says web + local web mod → RUN (no trip)"
TRACKED_WEB_BACKUP="$(mktemp)"
cp "$TRACKED_WEB" "$TRACKED_WEB_BACKUP"
printf '\n' >> "$TRACKED_WEB"   # tracked web edit → local heuristic sees web/
out_d="$(STUB_FILES='web/package.json' run_detect --pr 2862)"
cp "$TRACKED_WEB_BACKUP" "$TRACKED_WEB"   # restore immediately
rm -f "$TRACKED_WEB_BACKUP"; TRACKED_WEB_BACKUP=""
if [[ "$out_d" == "decision=RUN" ]]; then
  ok "agreeing signals run without tripping the guard ($out_d)"
else
  bad "expected 'decision=RUN' (no guard), got: $out_d"
fi

# ---------------------------------------------------------------------
# (e) --pr, gh query FAILS → fall back to local heuristic (SKIP here)
# ---------------------------------------------------------------------
note "Case (e): gh query fails → local fallback → SKIP"
out_e="$(STUB_RC=1 run_detect --pr 2863)"
if [[ "$out_e" == "decision=SKIP" ]]; then
  ok "gh outage degrades to local heuristic, does not crash ($out_e)"
else
  bad "expected decision=SKIP on gh failure, got: $out_e"
fi

echo
echo "smoke harness: $PASS pass / $FAIL fail"
[[ "$FAIL" -gt 0 ]] && exit 1
exit 0
