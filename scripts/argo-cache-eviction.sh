#!/usr/bin/env bash
# argo-cache-eviction.sh — evict oldest worker cargo target dirs when the
# shared cache exceeds a configurable threshold.
#
# Why this exists
# ---------------
# 2026-05-24 silent-fail incident on `codex-cli-routes-approvals-json-
# split-helpers`: worker reported "Build cache is huge (321GB, 836k
# files) — explains the slow clippy" after iter 2, then stalled 4h+ on
# cross-worker contention. Root cause: the shared cargo target dirs have
# no eviction policy. Every worker iteration adds compiled artifacts;
# nothing trims old per-brief target dirs after their worker exits.
#
# What this script DOES
# ---------------------
# 1. Measures total `du -sk` of known worker target dirs under $ROOT
#    (default /tmp): both /tmp/<brief>-target and
#    /tmp/argo-shared-target-<brief> naming conventions.
# 2. If total > threshold (default 150GB), lists those dirs sorted oldest-
#    first by mtime.
# 3. Skips the newest N (default 5) — workers in flight may still need
#    them. Also skips any dir whose corresponding worktree (/tmp/<brief>)
#    is currently on disk (active worker — its `claude -p` holds open
#    FDs into the target dir; pulling it would force a long rebuild).
# 4. Evicts the rest in order until total drops back under threshold.
#
# What this script DOES NOT do
# ----------------------------
# - Never touches ~/.cargo/registry/ (cross-iteration download cache;
#   expensive to re-fetch).
# - Never touches the SHARED global target dir (/tmp/argo-shared-target
#   or $HOME/.cache/argo-shared-target). Concurrent workers depend on it;
#   blowing it away forces a 28-min cold rebuild across the swarm.
# - Defaults to --dry-run. Pass --apply to actually `rm -rf`. The first
#   PR ships dry-run-as-default per the brief; flip to apply-by-default
#   in a follow-up after the operator confirms scoring on real /tmp.
#
# Env vars
# --------
#   ARGO_CACHE_EVICTION_GB_THRESHOLD   Trigger threshold in GB. Default 150.
#   ARGO_CACHE_EVICTION_RETAIN_NEWEST  Newest N dirs to preserve. Default 5.
#
# Lockfile
# --------
# Holds /tmp/argo-cache-eviction.lock for the duration of a run so two
# scheduled-task firings can't race. Stale locks (>1h old) are reclaimed.
#
# Exit codes
# ----------
#   0 — ok (no-op, dry-run, or successful eviction)
#   1 — eviction failed (rm error)
#   2 — invocation error (bad flag)
#   3 — another eviction in progress (lockfile held)
#
# Usage
# -----
#   scripts/argo-cache-eviction.sh                        # dry-run (default)
#   scripts/argo-cache-eviction.sh --apply                # actually evict
#   scripts/argo-cache-eviction.sh --root <dir> --apply   # test mode
#   scripts/argo-cache-eviction.sh --help

set -uo pipefail

THRESHOLD_GB="${ARGO_CACHE_EVICTION_GB_THRESHOLD:-150}"
RETAIN_NEWEST="${ARGO_CACHE_EVICTION_RETAIN_NEWEST:-5}"
LOCKFILE="/tmp/argo-cache-eviction.lock"
ROOT="/tmp"
MODE="dry-run"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h)  sed -n '2,55p' "$0" | sed 's/^# \?//'; exit 0 ;;
    --apply)    MODE="apply"; shift ;;
    --dry-run)  MODE="dry-run"; shift ;;
    --root)     ROOT="${2:-}"; shift 2 ;;
    --lockfile) LOCKFILE="${2:-}"; shift 2 ;;
    *)          echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

# Portable mtime → epoch (BSD/macOS + GNU).
mtime_epoch() {
  stat -f %m "$1" 2>/dev/null || stat -c %Y "$1" 2>/dev/null || echo 0
}

# Acquire the lockfile. Reclaim if it's >1h old (a prior run crashed
# before unlinking).
if [[ -e "$LOCKFILE" ]]; then
  age=$(( $(date +%s) - $(mtime_epoch "$LOCKFILE") ))
  if [[ "$age" -lt 3600 ]]; then
    echo "SKIP: lockfile $LOCKFILE held by pid $(cat "$LOCKFILE" 2>/dev/null) (age ${age}s)"
    exit 3
  fi
  echo "RECLAIM: stale lockfile $LOCKFILE age ${age}s; overwriting"
fi
echo "$$" > "$LOCKFILE"
trap 'rm -f "$LOCKFILE"' EXIT

# Scan known worker target dirs under $ROOT.
threshold_kb=$(( THRESHOLD_GB * 1024 * 1024 ))
total_kb=0
shopt -s nullglob
candidate_dirs=()
for d in "$ROOT"/*-target "$ROOT"/argo-shared-target-*; do
  [[ -d "$d" ]] || continue
  # Skip the global shared target dir (no per-brief suffix).
  [[ "$(basename "$d")" == "argo-shared-target" ]] && continue
  size=$(du -sk "$d" 2>/dev/null | awk '{print $1}')
  size=${size:-0}
  total_kb=$(( total_kb + size ))
  candidate_dirs+=("$size:$(mtime_epoch "$d"):$d")
done

total_gb=$(( total_kb / 1024 / 1024 ))
echo "MEASURE: ${total_gb}GB across ${#candidate_dirs[@]} brief-specific target dirs under $ROOT (threshold ${THRESHOLD_GB}GB)"

if [[ "$total_kb" -lt "$threshold_kb" ]]; then
  echo "OK: under threshold; no eviction needed"
  exit 0
fi

if [[ "${#candidate_dirs[@]}" -eq 0 ]]; then
  echo "OK: no brief-specific target dirs found under $ROOT; nothing to evict"
  exit 0
fi

# Sort candidates by mtime ascending (oldest first).
IFS=$'\n' read -r -d '' -a sorted < <(printf '%s\n' "${candidate_dirs[@]}" | sort -t: -k2 -n && printf '\0')

count=${#sorted[@]}
evict_count=$(( count - RETAIN_NEWEST ))
if [[ "$evict_count" -le 0 ]]; then
  echo "OK: only $count dirs (retain-newest=$RETAIN_NEWEST); nothing to evict"
  exit 0
fi

freed_kb=0
evicted=0
skipped_active=0
running_total=$total_kb
for (( i = 0; i < evict_count; i++ )); do
  entry="${sorted[$i]}"
  size_kb=$(echo "$entry" | cut -d: -f1)
  dir=$(echo "$entry" | cut -d: -f3-)
  brief=$(basename "$dir")
  brief=${brief%-target}
  brief=${brief#argo-shared-target-}

  # Safety: skip if the corresponding worktree is still on disk — an
  # active worker holds it open.
  if [[ -d "$ROOT/$brief" ]]; then
    echo "SKIP_ACTIVE: $dir (worktree $ROOT/$brief still present)"
    skipped_active=$(( skipped_active + 1 ))
    continue
  fi

  if [[ "$MODE" == "apply" ]]; then
    if rm -rf "$dir"; then
      echo "EVICTED: $dir (${size_kb}KB freed)"
    else
      echo "ERROR: rm -rf $dir failed" >&2
      exit 1
    fi
  else
    echo "DRY-RUN: would evict $dir (${size_kb}KB)"
  fi

  freed_kb=$(( freed_kb + size_kb ))
  running_total=$(( running_total - size_kb ))
  evicted=$(( evicted + 1 ))

  [[ "$running_total" -lt "$threshold_kb" ]] && break
done

freed_gb=$(( freed_kb / 1024 / 1024 ))
final_gb=$(( running_total / 1024 / 1024 ))
echo "SUMMARY: mode=$MODE evicted=$evicted skipped_active=$skipped_active freed=${freed_gb}GB final=${final_gb}GB"
