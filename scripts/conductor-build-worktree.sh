#!/usr/bin/env bash
# conductor-build-worktree.sh — give the conductor a DEDICATED build worktree
# + target dir so its cargo runs never contend with the worker swarm
# (DEC-042 Stage 1, B-1: shared-checkout race).
#
# Why: workers isolate brief CODE in /tmp/<brief-id> worktrees, but the
# conductor's own builds (conductor-pre-merge-gate.sh, restart-live-gateway.sh)
# run in the SHARED $REPO main checkout, sharing target dirs + the OS file
# cache with N compiling workers. This pegged RAM on 2026-06-09 (DEC-042 §1.3):
# a gateway rebuild collided with the workers and the swarm had to be PAUSED to
# build safely. "Pause the swarm to build" is a manual dance, not a fix.
#
# This script extends the worktree-isolation pattern that already works for
# brief code to the conductor's BUILDS: a reusable worktree at
# $HOME/.cache/argo-conductor-build with its own CARGO_TARGET_DIR at
# $HOME/.cache/argo-conductor-build-target. A conductor `cargo` run through it
# touches neither the shared $REPO tree nor the workers' shared target dir.
#
# Idempotent: creates the worktree on first call, reuses it after. Refreshes
# to a ref (default origin/main) via fetch + `reset --hard`. The reset is SAFE
# here — this is the conductor's OWN throwaway build tree, NEVER the shared
# $REPO checkout (which this script never touches except read-only `git`).
#
# Usage:
#   eval "$(bash scripts/conductor-build-worktree.sh)"            # refresh to origin/main, print exports
#   eval "$(bash scripts/conductor-build-worktree.sh --ref HEAD)" # sync to a specific ref/SHA
#   bash scripts/conductor-build-worktree.sh --no-refresh         # just print paths, no fetch/reset
#   bash scripts/conductor-build-worktree.sh --path               # print the worktree path only
#
# After eval, two variables are exported into the caller's shell:
#   CONDUCTOR_BUILD_WT      — the worktree path (run cargo from here)
#   CONDUCTOR_BUILD_TARGET  — the dedicated CARGO_TARGET_DIR
#
# Caller pattern (build the SAME commit the conductor has checked out, but in
# isolation — worktrees of one repo share the object DB, so a local SHA works):
#   eval "$(bash scripts/conductor-build-worktree.sh --ref "$(git rev-parse HEAD)")"
#   ( cd "$CONDUCTOR_BUILD_WT" && CARGO_TARGET_DIR="$CONDUCTOR_BUILD_TARGET" cargo ... )
#
# All human/log chatter goes to STDERR so STDOUT stays clean for `eval`.
#
# Exit codes:
#   0 = worktree ready; export lines printed to stdout
#   1 = could not create/refresh the worktree
#   2 = invalid argument

set -uo pipefail

REF="origin/main"
REFRESH=1
PRINT_PATH_ONLY=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --ref)        REF="${2:-}"; shift 2 ;;
    --ref=*)      REF="${1#*=}"; shift ;;
    --no-refresh) REFRESH=0; shift ;;
    --path)       PRINT_PATH_ONLY=1; shift ;;
    -h|--help)    grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *)            echo "conductor-build-worktree: unknown arg: $1" >&2; exit 2 ;;
  esac
done

log() { printf '[conductor-build-worktree] %s\n' "$*" >&2; }

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

WT_DIR="${CONDUCTOR_BUILD_WORKTREE_DIR:-$HOME/.cache/argo-conductor-build}"
WT_TARGET="${CONDUCTOR_BUILD_TARGET_DIR:-$HOME/.cache/argo-conductor-build-target}"

# Refuse a worktree dir under $REPO or /tmp — the whole point is an isolated,
# reboot-surviving build root on real disk separate from the shared tree.
case "$WT_DIR" in
  "$REPO_ROOT"|"$REPO_ROOT"/*)
    log "refusing worktree dir inside the shared repo ($WT_DIR); set CONDUCTOR_BUILD_WORKTREE_DIR"
    exit 1 ;;
  /tmp/*)
    log "refusing /tmp worktree dir ($WT_DIR) — does not survive reboot; set CONDUCTOR_BUILD_WORKTREE_DIR" ;;
esac

mkdir -p "$(dirname "$WT_DIR")" "$WT_TARGET" 2>/dev/null || true

# Is $WT_DIR already a registered worktree of this repo?
is_registered_worktree() {
  git -C "$REPO_ROOT" worktree list --porcelain 2>/dev/null \
    | grep -qxF "worktree $WT_DIR"
}

if ! is_registered_worktree; then
  # Clear a stale leftover directory that isn't a live worktree (e.g. a prune
  # left the dir but dropped the ref, or a previous partial create).
  if [ -e "$WT_DIR" ]; then
    log "stale path at $WT_DIR is not a registered worktree; removing"
    git -C "$REPO_ROOT" worktree remove --force "$WT_DIR" 2>/dev/null || rm -rf "$WT_DIR" 2>/dev/null || true
    git -C "$REPO_ROOT" worktree prune 2>/dev/null || true
  fi
  log "fetching origin/main before first worktree create"
  git -C "$REPO_ROOT" fetch origin main >/dev/null 2>&1 || log "warn: fetch failed; creating from local origin/main"
  log "creating build worktree at $WT_DIR (detached @ $REF)"
  if ! git -C "$REPO_ROOT" worktree add --detach "$WT_DIR" "$REF" >/dev/null 2>&1; then
    log "FAIL: git worktree add failed for $WT_DIR @ $REF"
    exit 1
  fi
elif [ "$REFRESH" -eq 1 ]; then
  log "refreshing existing worktree $WT_DIR to $REF"
  # Fetch only when refreshing to a remote-tracking ref; a local SHA/HEAD is
  # already in the shared object DB.
  case "$REF" in
    origin/*|refs/remotes/*) git -C "$REPO_ROOT" fetch origin main >/dev/null 2>&1 || log "warn: fetch failed; using local $REF" ;;
  esac
  if ! git -C "$WT_DIR" reset --hard "$REF" >/dev/null 2>&1; then
    log "FAIL: could not reset $WT_DIR to $REF"
    exit 1
  fi
  git -C "$WT_DIR" clean -ffdq 2>/dev/null || true
fi

WT_HEAD="$(git -C "$WT_DIR" rev-parse --short HEAD 2>/dev/null || echo unknown)"
log "ready: $WT_DIR @ $WT_HEAD  (CARGO_TARGET_DIR=$WT_TARGET)"

if [ "$PRINT_PATH_ONLY" -eq 1 ]; then
  printf '%s\n' "$WT_DIR"
  exit 0
fi

# Machine-parseable exports for `eval`.
printf 'export CONDUCTOR_BUILD_WT=%q\n' "$WT_DIR"
printf 'export CONDUCTOR_BUILD_TARGET=%q\n' "$WT_TARGET"
