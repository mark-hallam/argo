#!/usr/bin/env bash
# conductor-pre-merge-gate.sh — pre-merge gate the conductor runs locally
# while GitHub Actions remain operator-disabled.
#
# Why this exists: with GHA off, the only merge path is `gh pr merge --admin`.
# That bypasses CI workflows. Worker pre-push gates catch most lint/test
# regressions at push time, but a conductor admin-merge of a worker PR can
# silently land a regression introduced post-rebase. This script reproduces
# the critical workflow checks locally before the admin-merge fires.
#
# Per CLAUDE.md priority #1 (reliability) and audit finding 2026-05-18 RC5.
#
# Usage:
#   bash scripts/conductor-pre-merge-gate.sh                  # run all checks
#   bash scripts/conductor-pre-merge-gate.sh --skip-clippy    # skip workspace clippy (faster, lint-only PR)
#
# Exit codes:
#   0 = all gates pass; safe to admin-merge
#   1 = at least one gate failed; investigate before merging
#   2 = invalid argument / preflight failure

set -euo pipefail

SKIP_CLIPPY=0
for arg in "$@"; do
  case "$arg" in
    --skip-clippy) SKIP_CLIPPY=1 ;;
    -h|--help)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown flag: $arg" >&2
      exit 2
      ;;
  esac
done

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

fail_count=0
log()  { printf '\033[1m[pre-merge-gate]\033[0m %s\n' "$*"; }
warn() { printf '\033[33m[pre-merge-gate]\033[0m %s\n' "$*" >&2; }
fail() { printf '\033[31m[pre-merge-gate FAIL]\033[0m %s\n' "$*" >&2; fail_count=$((fail_count + 1)); }

log "running on $(git rev-parse --short HEAD)"

# Gate 1: migration slot collision check
log "[1/4] migration slot check"
if ! bash scripts/check-migration-slots.sh; then
  fail "migration slot collision detected — see output above"
fi

# Gate 2: LOC budget — no file > 500 LOC in crates/
log "[2/4] LOC budget (files > 500)"
over_500=$(find crates -name '*.rs' -exec wc -l {} \; 2>/dev/null | awk '$1 > 500 {print $0}')
if [[ -n "$over_500" ]]; then
  fail "files over 500 LOC ceiling:"
  echo "$over_500" >&2
fi

# Gate 3: parallel-pub-types canonicalization
log "[3/4] parallel-pub-types lint"
if ! python3 scripts/lint-parallel-pub-types.py >/dev/null 2>&1; then
  fail "parallel-pub-types violation — run python3 scripts/lint-parallel-pub-types.py"
fi

# Gate 4: workspace clippy (slowest; ~3-6 min). Skippable.
if [[ "$SKIP_CLIPPY" -eq 0 ]]; then
  log "[4/4] workspace clippy (3-6 min)"
  if ! bash scripts/workspace-clippy-fast-fail.sh; then
    fail "workspace clippy failure — see output above"
  fi
else
  warn "[4/4] workspace clippy SKIPPED (--skip-clippy)"
fi

echo
if [[ "$fail_count" -gt 0 ]]; then
  printf '\033[31mFAIL\033[0m %d gate(s) failed — DO NOT admin-merge.\n' "$fail_count" >&2
  exit 1
fi
printf '\033[32mPASS\033[0m all gates green — safe to admin-merge.\n'
exit 0
