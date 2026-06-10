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
#   bash scripts/conductor-pre-merge-gate.sh                  # run all 9 gates (gate 9 auto-skips without --pr)
#   bash scripts/conductor-pre-merge-gate.sh --skip-clippy    # skip workspace clippy (faster, lint-only PR)
#   bash scripts/conductor-pre-merge-gate.sh --skip-tests     # skip test-compile (no-Rust-touched PRs ONLY)
#   bash scripts/conductor-pre-merge-gate.sh --pr 2437        # also run gate 9 (PR-body anti-theater scan)
#   bash scripts/conductor-pre-merge-gate.sh --pr 2437 --allow-anti-theater-admit
#                                                             # override gate 9 + log to SHARED.md
#
# Gate 7 (web typecheck) auto-skips when the diff touches no web/** files,
# so Rust-only PRs pay zero cost. It runs `pnpm tsc --noEmit` against the
# SPA workspace; closes the gap that let TS regressions (#2404 + #2366)
# land on main and only surface at the next evaluator round.
#
# Gate 9 (PR-body anti-theater scan) auto-skips when --pr is omitted, so
# all existing local-only invocations and the conductor-pre-merge-gate-tests.sh
# smoke harness stay green. When --pr is given, the gate refuses to PASS
# if the PR body contains a recognized "test was not run" admission
# phrase. --allow-anti-theater-admit is the operator override (logs to
# docs/skycot/agents/SHARED.md for the next evaluator round to verify).
# Origin: R97 (2026-05-27) — PR #2437 admin-merged after the body line
# "Local cargo check was interrupted by file-lock contention" and broke
# its own regression test.
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
LOC_CEILING=500
PR_NUMBER=""
ALLOW_ANTI_THEATER=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-clippy) SKIP_CLIPPY=1; shift ;;
    --skip-tests)  SKIP_TESTS=1; shift ;;
    --allow-loc-overage=*) LOC_CEILING="${1#*=}"; shift ;;
    --pr)
      PR_NUMBER="${2:-}"
      shift 2
      ;;
    --pr=*)
      PR_NUMBER="${1#*=}"
      shift
      ;;
    --allow-anti-theater-admit)
      ALLOW_ANTI_THEATER=1
      shift
      ;;
    -h|--help)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown flag: $1" >&2
      exit 2
      ;;
  esac
done

# Hard cap on LOC_CEILING (operator-locked slippery-slope guard per
# OVERNIGHT DIRECTIVE §10 / memory
# feedback_loc_overage_550_soft_ceiling_mvp_window_2026_05_25.md).
if [[ "$LOC_CEILING" -gt 550 ]]; then
  echo "[pre-merge-gate FAIL] --allow-loc-overage=$LOC_CEILING exceeds 550 hard cap" >&2
  exit 2
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

fail_count=0
log()  { printf '\033[1m[pre-merge-gate]\033[0m %s\n' "$*"; }
warn() { printf '\033[33m[pre-merge-gate]\033[0m %s\n' "$*" >&2; }
fail() { printf '\033[31m[pre-merge-gate FAIL]\033[0m %s\n' "$*" >&2; fail_count=$((fail_count + 1)); }

log "running on $(git rev-parse --short HEAD)"

# DEC-042 B-1: optionally run the two cargo-heavy gates (test-compile + workspace
# clippy) in a DEDICATED build worktree + target dir so they never contend with
# the worker swarm's shared checkout/target. This is what removes the manual
# "pause the swarm to build" dance (the 2026-06-09 failure). Default OFF — set
# ARGO_CONDUCTOR_BUILD_WORKTREE=1 to enable. When enabled, the worktree is synced
# to THIS gate's current HEAD (worktrees of one repo share the object DB, so a
# local SHA builds the exact same tree, just in isolation). Falls back to the
# in-place build if the helper is absent or fails — no behavior change otherwise.
GATE_SCRIPTS_ROOT="$repo_root"   # where the cargo-gate helper scripts are run from
if [[ "${ARGO_CONDUCTOR_BUILD_WORKTREE:-0}" == "1" && -x "$repo_root/scripts/conductor-build-worktree.sh" ]]; then
  head_sha="$(git rev-parse HEAD)"
  if eval "$("$repo_root/scripts/conductor-build-worktree.sh" --ref "$head_sha" 2>/dev/null)" \
       && [[ -n "${CONDUCTOR_BUILD_WT:-}" && -d "$CONDUCTOR_BUILD_WT" ]]; then
    GATE_SCRIPTS_ROOT="$CONDUCTOR_BUILD_WT"
    export CARGO_TARGET_DIR="$CONDUCTOR_BUILD_TARGET"
    log "build isolation ON — cargo gates run in $CONDUCTOR_BUILD_WT (target $CARGO_TARGET_DIR)"
  else
    warn "conductor-build-worktree.sh unavailable/failed; falling back to in-place build"
  fi
fi

# Fast preflight: catch the v83-class failure where a migration lands
# without bumping the runtime EXPECTED_SCHEMA_VERSION constant.
log "[preflight] expected schema version matches tip migration"
if ! bash scripts/check-expected-schema-version-matches-tip.sh; then
  fail "EXPECTED_SCHEMA_VERSION does not match the tip migration — see output above"
fi

# Gate 1: gateway-boot tests need a SQLCipher key. Preserve any
# operator-supplied key; otherwise generate an ephemeral per-run key.
log "[1/9] ensure SKYCOT_DB_KEY_BASE64"
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
log "[2/9] migration slot check"
if ! bash scripts/check-migration-slots.sh; then
  fail "migration slot collision detected — see output above"
fi

# Gate 3: LOC budget — files > LOC_CEILING (default 500, max 550) hard-fail;
# files in (500, LOC_CEILING] log WARN (tracked overage per OVERNIGHT DIRECTIVE §10).
log "[3/9] LOC budget (hard ceiling $LOC_CEILING; soft ceiling 500)"
over_ceiling=$(find crates -name '*.rs' -exec wc -l {} \; 2>/dev/null | awk -v cap="$LOC_CEILING" '$1 > cap {print $0}')
if [[ -n "$over_ceiling" ]]; then
  fail "files over $LOC_CEILING LOC hard ceiling:"
  echo "$over_ceiling" >&2
fi
if [[ "$LOC_CEILING" -gt 500 ]]; then
  over_500_under_cap=$(find crates -name '*.rs' -exec wc -l {} \; 2>/dev/null | awk -v cap="$LOC_CEILING" '$1 > 500 && $1 <= cap {print $0}')
  if [[ -n "$over_500_under_cap" ]]; then
    warn "LOC overage 500-${LOC_CEILING} (tracked at docs/skycot/agents/heartbeats/loc-overage-tracking.md):"
    echo "$over_500_under_cap" >&2
  fi
fi

# Gate 4: parallel-pub-types canonicalization
log "[4/9] parallel-pub-types lint"
if ! python3 scripts/lint-parallel-pub-types.py --enforce >/dev/null 2>&1; then
  fail "parallel-pub-types violation — run python3 scripts/lint-parallel-pub-types.py"
fi

# Gate 5: zero-caller substrate — enforce CLAUDE.md "wire or defer"
# at merge time. Cheap (one cargo metadata + bounded grep over
# workspace src/), runs ahead of the 3-6min workspace clippy so
# wire-or-defer drift surfaces first.
log "[5/9] zero-caller substrate lint"
if ! bash scripts/check-zero-caller-substrate.sh; then
  fail "zero-caller substrate lint — a crate has no \`use <name>::\` callers AND no zero-caller-allowed marker"
fi

# Gate 5b: gateway /routes catalog completeness — F8 + F28 of
# ux-e2e-audit-T2309MEL (2026-05-26). Static-grep gate that confirms
# every `/api/v1/<segment>` family the dispatchers register also
# appears in `routes_catalog.rs`. Pure Python+grep, runs in <1s, no
# gateway boot required.
log "[5b/9] gateway /routes catalog completeness"
if ! bash scripts/check-routes-catalog-complete.sh; then
  fail "routes-catalog drift — a dispatcher-registered /api/v1/* family is missing from routes_catalog.rs"
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
  log "[6/9] test-compile (cargo check --workspace --tests; ~30s warm)"
  if ! bash "$GATE_SCRIPTS_ROOT/scripts/pre-merge-test-compile.sh"; then
    fail "test-compile gate FAILED — see output above"
  fi
else
  warn ""
  warn "================================================================"
  warn "[6/9] test-compile SKIPPED (--skip-tests)"
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

# Gate 7: web typecheck — runs `pnpm tsc --noEmit` on the SPA workspace
# IF the diff touches any web/** file. The helper auto-skips for
# Rust-only PRs (zero cost). Closes the gap that let #2404 + #2366 TS
# regressions land on main between evaluator rounds.
log "[7/9] web typecheck (auto-skips if no web/** files touched)"
if ! bash scripts/pre-merge-web-typecheck.sh; then
  fail "web typecheck FAILED — see output above"
fi

# Gate 8: workspace clippy (slowest; ~3-6 min). Skippable.
if [[ "$SKIP_CLIPPY" -eq 0 ]]; then
  log "[8/9] workspace clippy (3-6 min)"
  if ! bash "$GATE_SCRIPTS_ROOT/scripts/workspace-clippy-fast-fail.sh"; then
    fail "workspace clippy failure — see output above"
  fi
else
  warn "[8/9] workspace clippy SKIPPED (--skip-clippy)"
fi

# Gate 9: PR-body anti-theater scan. Cheap (~2s; one `gh pr view` call)
# and runs last so the operator sees ONE summary warning if it fires
# rather than eight gate banners. Auto-skips when --pr is omitted, so
# all existing local-only invocations stay green. Origin: R97
# (2026-05-27) — PR #2437 admin-merged with body line "Local cargo
# check was interrupted by file-lock contention" and broke its own
# regression test. The scan reads what the worker wrote BEFORE merge,
# rather than relying on the next evaluator round to catch the
# admission post-merge.
log "[9/9] PR body anti-theater scan"
gate9_args=()
if [[ -n "$PR_NUMBER" ]]; then
  gate9_args+=(--pr "$PR_NUMBER")
fi
if [[ "$ALLOW_ANTI_THEATER" -eq 1 ]]; then
  gate9_args+=(--allow-anti-theater-admit)
fi
# `${arr[@]+"${arr[@]}"}` is the bash 3.2-compatible idiom for
# expanding an array only when it has elements — bare "${arr[@]}"
# trips `set -u` on macOS bash 3.2 when the array is empty.
if ! bash scripts/pre-merge-pr-body-anti-theater-scan.sh ${gate9_args[@]+"${gate9_args[@]}"}; then
  fail "PR body contains anti-theater admission — see output above"
fi

echo
if [[ "$fail_count" -gt 0 ]]; then
  printf '\033[31mFAIL\033[0m %d gate(s) failed — DO NOT admin-merge.\n' "$fail_count" >&2
  exit 1
fi
printf '\033[32mPASS\033[0m all gates green — safe to admin-merge.\n'
exit 0
