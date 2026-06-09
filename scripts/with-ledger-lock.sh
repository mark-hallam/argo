#!/usr/bin/env bash
# with-ledger-lock.sh — portable per-host mutex around the BUILD_STATUS.md
# read-modify-write (DEC-042 Stage 1, B-1: shared-checkout ledger race).
#
# Why: workers + the conductor all `git pull --rebase` → edit
# `docs/build-plan/BUILD_STATUS.md` → commit → `git push origin main` in the
# SAME shared `$REPO` main checkout. Line-disjoint edits usually rebase
# cleanly, but same-row edits collide and the watchdog's mid-edit stash can
# orphan a brief mid-write — producing divergence flakes and the occasional
# clobbered status row (DEC-042 §3 B-1). Serializing those critical sections
# behind one host-local lock removes the collision window.
#
# Portability: macOS does NOT ship flock(1) (util-linux). This uses an
# atomic `mkdir` spinlock with a stale-lock reaper (a lock whose holder PID is
# dead is reclaimed), so it works on stock macOS + Linux with no extra deps.
#
# ADDITIVE SAFETY — never a hard block. If the lock directory cannot be
# created for ANY reason (unwritable .git, disk full, race we lose past the
# timeout), this falls through and runs the command ANYWAY. A worker must
# never deadlock waiting on a ledger lock; the existing `pull --rebase`
# fallback inside the wrapped command is the second line of defence.
#
# Two ways to use it:
#
#   1. As a command wrapper (the documented interface):
#        bash scripts/with-ledger-lock.sh -- git push origin main
#        bash scripts/with-ledger-lock.sh -- bash -c 'edit; commit; push'
#
#   2. Sourced, to wrap an in-process shell function (so callers can serialize
#      a multi-statement block without a subprocess):
#        . scripts/with-ledger-lock.sh
#        ledger_lock_run my_commit_push_function "arg"
#
# Environment:
#   LEDGER_LOCK_DIR     — lock directory path. Default:
#                         "$(git rev-parse --git-common-dir)/skycot-ledger.lock"
#                         (shared by every worktree of the same repo, so all
#                         workers + the conductor on one host agree on it).
#   LEDGER_LOCK_TIMEOUT — seconds to spin before falling through unlocked.
#                         Default 60.
#   LEDGER_LOCK_DISABLE — set to 1 to skip locking entirely (run direct).
#
# Exit code: the wrapped command's exit code (or 2 for a usage error).

# --- lock primitives (sourced-safe: no `set -e`, no top-level side effects) ---

# _ledger_lock_dir — resolve the lock directory path once.
_ledger_lock_dir() {
  if [ -n "${LEDGER_LOCK_DIR:-}" ]; then
    printf '%s\n' "$LEDGER_LOCK_DIR"
    return 0
  fi
  local common
  common="$(git rev-parse --git-common-dir 2>/dev/null)"
  if [ -n "$common" ]; then
    # git-common-dir can be relative to cwd; resolve against it.
    case "$common" in
      /*) printf '%s/skycot-ledger.lock\n' "$common" ;;
      *)  printf '%s/%s/skycot-ledger.lock\n' "$(pwd)" "$common" ;;
    esac
    return 0
  fi
  # Last resort: a host-temp path keyed on the repo, so concurrent processes
  # still converge even outside a git tree.
  printf '%s/skycot-ledger.lock\n' "${TMPDIR:-/tmp}"
}

# _ledger_lock_acquire <lockdir> <timeout> — returns 0 if acquired (caller
# owns it + must release), 1 if it gave up / could not lock (caller proceeds
# UNLOCKED — additive-safety fall-through).
_ledger_lock_acquire() {
  local lockdir="$1" timeout="$2"
  local waited=0
  while true; do
    if mkdir "$lockdir" 2>/dev/null; then
      echo "$$" > "$lockdir/pid" 2>/dev/null || true
      return 0
    fi
    # Lock held — reap it if the holder PID is dead (stale lock).
    local holder
    holder="$(cat "$lockdir/pid" 2>/dev/null || echo '')"
    if [ -n "$holder" ] && ! kill -0 "$holder" 2>/dev/null; then
      # Holder is gone; reclaim. rm+retry (another waiter may win the retry).
      rm -rf "$lockdir" 2>/dev/null || true
      continue
    fi
    if [ "$waited" -ge "$timeout" ]; then
      echo "with-ledger-lock: timeout after ${timeout}s waiting on $lockdir (holder=${holder:-unknown}); proceeding UNLOCKED" >&2
      return 1
    fi
    sleep 1
    waited=$((waited + 1))
  done
}

# _ledger_lock_release <lockdir> — drop the lock iff we still own it.
_ledger_lock_release() {
  local lockdir="$1"
  local holder
  holder="$(cat "$lockdir/pid" 2>/dev/null || echo '')"
  if [ "$holder" = "$$" ]; then
    rm -rf "$lockdir" 2>/dev/null || true
  fi
}

# ledger_lock_run <command...> — acquire, run the command (which may be a
# shell function when this file is sourced), release on the way out. Falls
# through to a direct run when locking is disabled or unavailable.
ledger_lock_run() {
  local rc=0
  if [ "${LEDGER_LOCK_DISABLE:-0}" = "1" ]; then
    "$@" || rc=$?
    return $rc
  fi
  local lockdir timeout acquired=0
  lockdir="$(_ledger_lock_dir)"
  timeout="${LEDGER_LOCK_TIMEOUT:-60}"
  # mkdir -p the PARENT so the lock dir itself is the only atomic create.
  mkdir -p "$(dirname "$lockdir")" 2>/dev/null || true
  if _ledger_lock_acquire "$lockdir" "$timeout"; then
    acquired=1
  fi
  # `|| rc=$?` keeps the wrapped command's exit code AND stays safe under a
  # caller's `set -e` (the command failing must not skip the release below).
  "$@" || rc=$?
  [ "$acquired" -eq 1 ] && _ledger_lock_release "$lockdir"
  return $rc
}

# --- executed-as-a-script entrypoint --------------------------------------
# Only runs main() when invoked directly; sourcing exposes the helpers above
# without acquiring a lock or parsing args.
_with_ledger_lock_main() {
  # Strip a single leading `--` separator if present.
  if [ "${1:-}" = "--" ]; then
    shift
  fi
  if [ "$#" -eq 0 ]; then
    echo "usage: $0 [--] <command> [args...]" >&2
    return 2
  fi
  ledger_lock_run "$@"
}

if [ "${BASH_SOURCE[0]:-$0}" = "${0}" ]; then
  set -uo pipefail
  _with_ledger_lock_main "$@"
  exit $?
fi
