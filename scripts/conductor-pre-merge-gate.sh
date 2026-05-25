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
#   bash scripts/conductor-pre-merge-gate.sh                  # run all 7 gates
#   bash scripts/conductor-pre-merge-gate.sh --skip-clippy    # skip workspace clippy (faster, lint-only PR)
#   bash scripts/conductor-pre-merge-gate.sh --skip-tests     # skip test-compile (no-Rust-touched PRs ONLY)
#
# Flag semantics:
#   --skip-clippy is the "lint is green; I just want fast iteration" shortcut.
#   It does NOT skip the test-compile gate — that gate exists precisely
#   because --skip-clippy used to mask `cargo test` red.
#
#   --skip-tests is the no-Rust-touched escape hatch (docs / ledger / markdown
#   PRs). It emits a loud warning and is intended ONLY when the PR genuinely
#   touches zero .rs files. Abuse will land broken tests on main.
#
# Exit codes:
#   0 = all gates pass; safe to admin-merge
#   1 = at least one gate failed; investigate before merging
#   2 = invalid argument / preflight failure

set -euo pipefail

SKIP_CLIPPY=0
SKIP_TESTS=0
for arg in "$@"; do
  case "$arg" in
    --skip-clippy) SKIP_CLIPPY=1 ;;
    --skip-tests)  SKIP_TESTS=1 ;;
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

# Gate 1: gateway-boot tests need a SQLCipher key. Preserve any
# operator-supplied key; otherwise generate an ephemeral per-run key.
log "[1/7] ensure SKYCOT_DB_KEY_BASE64"
if [[ -z "${SKYCOT_DB_KEY_BASE64:-}" ]]; then
  if ! command -v openssl >/dev/null 2>&1; then
    printf '\033[31m[pre-merge-gate FAIL]\033[0m openssl not found; install with: brew install openssl\n' >&2
    exit 2
  fi
  export SKYCOT_DB_KEY_BASE64="$(openssl rand -base64 32)"
else
  export SKYCOT_DB_KEY_BASE64
fi

# Gate 2: migration slot collision check
log "[2/7] migration slot check"
if ! bash scripts/check-migration-slots.sh; then
  fail "migration slot collision detected — see output above"
fi

# Gate 3: LOC budget — no file > 500 LOC in crates/
log "[3/7] LOC budget (files > 500)"
over_500=$(find crates -name '*.rs' -exec wc -l {} \; 2>/dev/null | awk '$1 > 500 {print $0}')
if [[ -n "$over_500" ]]; then
  fail "files over 500 LOC ceiling:"
  echo "$over_500" >&2
fi

# Gate 4: parallel-pub-types canonicalization
log "[4/7] parallel-pub-types lint"
if ! python3 scripts/lint-parallel-pub-types.py >/dev/null 2>&1; then
  fail "parallel-pub-types violation — run python3 scripts/lint-parallel-pub-types.py"
fi

# Gate 5: zero-caller substrate — enforce CLAUDE.md "wire or defer"
# at merge time. Cheap (one cargo metadata + bounded grep over
# workspace src/), runs ahead of the 3-6min workspace clippy so
# wire-or-defer drift surfaces first.
log "[5/7] zero-caller substrate lint"
if ! bash scripts/check-zero-caller-substrate.sh; then
  fail "zero-caller substrate lint — a crate has no \`use <name>::\` callers AND no zero-caller-allowed marker"
fi

# Gate 6: test-compile (cargo check --workspace --tests). Honored
# allowlist at scripts/pre-merge-test-carry-allowlist.txt permits
# explicit carries. Runs BEFORE clippy so cross-crate test breakage
# surfaces without waiting on the 3-6min workspace clippy step.
#
# NOT skipped by --skip-clippy. Use --skip-tests (loud warning) for
# no-Rust-touched PRs only. Audit gap that motivated this gate:
# rounds 41-53 evaluator carry-forward (6 rounds) of `--skip-clippy`
# being the only cross-crate sanity check, letting `cargo test` red
# slip through.
if [[ "$SKIP_TESTS" -eq 0 ]]; then
  log "[6/7] test-compile (cargo check --workspace --tests; ~30s warm)"
  if ! bash scripts/pre-merge-test-compile.sh; then
    fail "test-compile gate FAILED — see output above"
  fi
else
  warn ""
  warn "================================================================"
  warn "[6/7] test-compile SKIPPED (--skip-tests)"
  warn ""
  warn "This bypass is intended ONLY for PRs that touch zero .rs files"
  warn "(docs / ledger / markdown). If your PR touches any Rust source,"
  warn "you are playing with fire — landing broken cargo test on main"
  warn "is the exact failure mode this gate exists to prevent."
  warn ""
  warn "Justify the bypass in the PR body (one line)."
  warn "================================================================"
  warn ""
fi

# Gate 7: workspace clippy (slowest; ~3-6 min). Skippable.
if [[ "$SKIP_CLIPPY" -eq 0 ]]; then
  log "[7/7] workspace clippy (3-6 min)"
  if ! bash scripts/workspace-clippy-fast-fail.sh; then
    fail "workspace clippy failure — see output above"
  fi
else
  warn "[7/7] workspace clippy SKIPPED (--skip-clippy)"
fi

echo
if [[ "$fail_count" -gt 0 ]]; then
  printf '\033[31mFAIL\033[0m %d gate(s) failed — DO NOT admin-merge.\n' "$fail_count" >&2
  exit 1
fi
printf '\033[32mPASS\033[0m all gates green — safe to admin-merge.\n'
exit 0
