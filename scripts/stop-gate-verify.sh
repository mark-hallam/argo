#!/usr/bin/env bash
# stop-gate-verify.sh — DEC-042 Stage-1 B-2: the single CHECKABLE verifier that
# "done" binds to. It COMPOSES the existing gate scripts (it does NOT author a
# parallel gate) over a worker's pushed branch, running the cargo-heavy steps in
# the B-1 conductor build worktree so the verify never contends with the worker
# swarm's RAM. Behind the default-OFF flag ARGO_STOP_GATE (DEC-042 §3 B-2).
#
# Why this exists: today verification is INSTRUCTED in the worker prompt
# (claude-worker-loop.sh) but never ENFORCED — a session can skip the gate and
# still flip its ledger row to IN_REVIEW. This is the NO-THEATER (DEC-029) hole
# compiled into the build loop. This script is the verifier; the load-bearing
# enforcement that consumes it is scripts/stop-gate-enforce.sh.
#
# Usage:
#   ARGO_STOP_GATE=1 scripts/stop-gate-verify.sh --brief <id> --branch <branch> \
#       [--worktree <path>] [--repo <path>]
#
# Output contract (last line is machine-parseable):
#   STOP_GATE_PASS
#   STOP_GATE_FAIL: <one-line reason>
#
# Exit codes:
#   0 = gate passed, OR gate disabled (ARGO_STOP_GATE != 1) — no-op
#   1 = gate failed (reason printed on the STOP_GATE_FAIL line)
#   2 = invalid arguments / preflight error
#
# Gate composition (fail-fast, cheap -> expensive):
#   1. LOC budget        (no compile)            — REUSES the pre-merge LOC rule
#   2. ledger-in-band    (no compile)            — FEATURE_MATRIX.md <-> .html sync
#   3. cargo test        (touched crates only)
#   4. workspace clippy  (cross-crate drift)     — REUSES workspace-clippy-fast-fail.sh
#
# Each gate command is OVERRIDABLE via env so scripts/test-stop-gate.sh can
# inject a deterministic pass/fail with NO real compile:
#   STOP_GATE_LOC_CMD  STOP_GATE_LEDGER_CMD  STOP_GATE_TEST_CMD  STOP_GATE_CLIPPY_CMD
# An override is run with `bash -c "<cmd>"` from the resolved tree; non-zero =
# that gate failed.

set -uo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
BRIEF=""
BRANCH=""
WORKTREE=""
LOC_CEILING="${STOP_GATE_LOC_CEILING:-500}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --brief)     BRIEF="${2:-}"; shift 2 ;;
    --brief=*)   BRIEF="${1#*=}"; shift ;;
    --branch)    BRANCH="${2:-}"; shift 2 ;;
    --branch=*)  BRANCH="${1#*=}"; shift ;;
    --worktree)  WORKTREE="${2:-}"; shift 2 ;;
    --worktree=*) WORKTREE="${1#*=}"; shift ;;
    --repo)      REPO="${2:-}"; shift 2 ;;
    --repo=*)    REPO="${1#*=}"; shift ;;
    -h|--help)   grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *)           echo "stop-gate-verify: unknown arg: $1" >&2; exit 2 ;;
  esac
done

# DISABLED no-op: with the flag unset, behavior is byte-identical to pre-B-2.
# This is half of the "no-behavior-change proof" (the other half is the
# default-OFF guard at the wrapper call site).
if [[ "${ARGO_STOP_GATE:-0}" != "1" ]]; then
  echo "STOP_GATE_PASS"
  exit 0
fi

if [[ -z "$BRIEF" ]]; then
  echo "stop-gate-verify: --brief is required" >&2
  echo "STOP_GATE_FAIL: missing --brief"
  exit 2
fi

fail() { echo "STOP_GATE_FAIL: $1"; exit 1; }

LOC_OVERRIDE="${STOP_GATE_LOC_CMD:-}"
LEDGER_OVERRIDE="${STOP_GATE_LEDGER_CMD:-}"
TEST_OVERRIDE="${STOP_GATE_TEST_CMD:-}"
CLIPPY_OVERRIDE="${STOP_GATE_CLIPPY_CMD:-}"

# Do we need the branch diff (and therefore a real tree / build worktree)?
# The LOC + ledger + cargo defaults all consult it (LOC grades only the files
# THIS branch changed — a pre-existing over-budget file the worker never touched
# is not this change's failure). If every consulting gate is overridden (the
# test path), we touch no git and no worktree at all.
need_diff=0
[[ -z "$LOC_OVERRIDE"    ]] && need_diff=1
[[ -z "$LEDGER_OVERRIDE" ]] && need_diff=1
[[ -z "$TEST_OVERRIDE"   ]] && need_diff=1
[[ -z "$CLIPPY_OVERRIDE" ]] && need_diff=1

TREE="$REPO"
TARGET="${CARGO_TARGET_DIR:-}"
CHANGED=""
CRATE_FLAGS=""

if [[ "$need_diff" -eq 1 ]]; then
  # Prefer the B-1 conductor build worktree (isolated tree + target dir) synced
  # to the pushed branch tip, so cargo never pegs the workers' shared target.
  if [[ "${ARGO_CONDUCTOR_BUILD_WORKTREE:-0}" == "1" && -x "$REPO/scripts/conductor-build-worktree.sh" && -n "$BRANCH" ]]; then
    git -C "$REPO" fetch origin "$BRANCH" >/dev/null 2>&1 || true
    branch_sha="$(git -C "$REPO" rev-parse FETCH_HEAD 2>/dev/null || echo "")"
    if [[ -n "$branch_sha" ]] \
         && eval "$("$REPO/scripts/conductor-build-worktree.sh" --ref "$branch_sha" 2>/dev/null)" \
         && [[ -n "${CONDUCTOR_BUILD_WT:-}" && -d "$CONDUCTOR_BUILD_WT" ]]; then
      TREE="$CONDUCTOR_BUILD_WT"
      TARGET="$CONDUCTOR_BUILD_TARGET"
    fi
  elif [[ -n "$WORKTREE" && -d "$WORKTREE" ]]; then
    TREE="$WORKTREE"
  fi

  base="$(git -C "$TREE" merge-base origin/main HEAD 2>/dev/null || echo "")"
  if [[ -n "$base" ]]; then
    CHANGED="$(git -C "$TREE" diff --name-only "$base"...HEAD 2>/dev/null || echo "")"
  else
    CHANGED="$(git -C "$TREE" diff --name-only HEAD 2>/dev/null || echo "")"
  fi
  crates="$(printf '%s\n' "$CHANGED" | sed -n 's#^crates/\([^/]*\)/.*#\1#p' | sort -u)"
  for c in $crates; do CRATE_FLAGS="$CRATE_FLAGS -p $c"; done
fi

# ---- Gate 1: LOC budget (no compile) -------------------------------------
# Grades ONLY the crates/*.rs files THIS branch changed — a pre-existing
# over-budget file the worker never touched is not this change's failure
# (flagging it would be a false-positive, i.e. its own kind of theater).
echo "[stop-gate] [1/4] LOC budget (ceiling $LOC_CEILING; changed files only)" >&2
if [[ -n "$LOC_OVERRIDE" ]]; then
  ( cd "$TREE" && bash -c "$LOC_OVERRIDE" ) || fail "LOC budget gate (override) failed"
else
  while IFS= read -r f; do
    [[ -z "$f" ]] && continue
    case "$f" in crates/*) [[ "$f" == *.rs ]] || continue ;; *) continue ;; esac
    [[ -f "$TREE/$f" ]] || continue   # deleted/renamed-away path: nothing to grade
    n="$(wc -l < "$TREE/$f" 2>/dev/null | tr -d ' ')"
    if [[ -n "$n" && "$n" -gt "$LOC_CEILING" ]]; then
      fail "LOC budget: changed file over $LOC_CEILING LOC — $f ($n LOC)"
    fi
  done <<< "$CHANGED"
fi

# ---- Gate 2: ledger-in-band (no compile) ---------------------------------
# CLAUDE.md rule: whenever FEATURE_MATRIX.md changes, feature-matrix.html must be
# regenerated in the SAME commit. Read-only co-presence check over the branch
# diff (safe in any tree — never mutates).
echo "[stop-gate] [2/4] ledger-in-band (FEATURE_MATRIX.md <-> feature-matrix.html)" >&2
if [[ -n "$LEDGER_OVERRIDE" ]]; then
  ( cd "$TREE" && bash -c "$LEDGER_OVERRIDE" ) || fail "ledger-in-band gate (override) failed"
else
  if printf '%s\n' "$CHANGED" | grep -qx "docs/build-plan/FEATURE_MATRIX.md"; then
    if ! printf '%s\n' "$CHANGED" | grep -qx "docs/skycot/feature-matrix.html"; then
      fail "ledger-in-band: FEATURE_MATRIX.md changed but docs/skycot/feature-matrix.html was not regenerated in the same branch (run python3 scripts/render-feature-matrix.py)"
    fi
  fi
fi

# ---- Gate 3: cargo test for touched crates -------------------------------
echo "[stop-gate] [3/4] cargo test ($CRATE_FLAGS)" >&2
if [[ -n "$TEST_OVERRIDE" ]]; then
  ( cd "$TREE" && bash -c "$TEST_OVERRIDE" ) || fail "cargo test gate (override) failed"
elif [[ -z "$CRATE_FLAGS" ]]; then
  echo "[stop-gate] no crates touched — cargo test skipped" >&2
else
  # CRATE_FLAGS is intentionally word-split into multiple `-p <crate>` args.
  # shellcheck disable=SC2086
  if ! ( cd "$TREE" && CARGO_TARGET_DIR="$TARGET" cargo test $CRATE_FLAGS 2>&1 | tail -60 ); then
    fail "cargo test failed for$CRATE_FLAGS"
  fi
fi

# ---- Gate 4: workspace clippy (cross-crate drift) ------------------------
echo "[stop-gate] [4/4] workspace clippy (cross-crate drift)" >&2
if [[ -n "$CLIPPY_OVERRIDE" ]]; then
  ( cd "$TREE" && bash -c "$CLIPPY_OVERRIDE" ) || fail "workspace clippy gate (override) failed"
elif [[ -z "$CRATE_FLAGS" ]]; then
  echo "[stop-gate] no crates touched — workspace clippy skipped" >&2
else
  if ! ( cd "$TREE" && CARGO_TARGET_DIR="$TARGET" bash "$TREE/scripts/workspace-clippy-fast-fail.sh" --quiet ); then
    fail "workspace clippy failed (cross-crate drift or lint error)"
  fi
fi

echo "STOP_GATE_PASS"
exit 0
