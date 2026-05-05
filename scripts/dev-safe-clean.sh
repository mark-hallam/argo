#!/usr/bin/env bash
# Dev runtime safe cleanup.
#
# Reclaims disk space from the four known build/runtime
# artifact locations:
#
#   - target/                  (cargo build cache)
#   - .skycot/outputs/         (per-job worker output bundles)
#   - /private/tmp/argo*       (argo-side scratch tempdirs)
#   - /private/tmp/claude-*    (claude code runner tempfiles)
#
# Safety constraints (non-negotiable):
#
#   - Defaults to dry-run; --apply is required to delete.
#   - Every path passes through a strict whitelist check.
#     Anything outside the four prefixes above is REFUSED with
#     a clear error.
#   - Empty / unset path variables are rejected before any
#     `rm` runs.
#   - Source files, .env, configs, the memory DB, and the
#     workspace's argo.toml are never touched.
#   - Prints exactly what will be / was deleted.
#
# Retention policy:
#
#   - Outputs newer than --max-age-days (default 14) are kept.
#   - The newest --keep-recent (default 20) outputs are kept
#     even if older than the age cutoff (so a bursty CI run
#     doesn't drop history that's still being reviewed).
#   - target/, /private/tmp/argo*, /private/tmp/claude-* are
#     deleted wholesale when --apply is set; they're transient.
#
# Usage:
#   ./scripts/dev-safe-clean.sh                   # dry-run
#   ./scripts/dev-safe-clean.sh --apply           # actually delete
#   ./scripts/dev-safe-clean.sh --apply --max-age-days 7
#   ./scripts/dev-safe-clean.sh --apply --keep-recent 50
#   ./scripts/dev-safe-clean.sh --skip-target     # leave cargo cache alone
#
# Exit codes:
#   0   success
#   1   safety violation (refused path, missing prereq, etc.)

set -uo pipefail

APPLY=false
MAX_AGE_DAYS=14
KEEP_RECENT=20
SKIP_TARGET=false
SKIP_OUTPUTS=false
SKIP_TMP=false

while [ $# -gt 0 ]; do
  case "$1" in
    --apply)            APPLY=true; shift ;;
    --max-age-days)     MAX_AGE_DAYS="$2"; shift 2 ;;
    --keep-recent)      KEEP_RECENT="$2"; shift 2 ;;
    --skip-target)      SKIP_TARGET=true; shift ;;
    --skip-outputs)     SKIP_OUTPUTS=true; shift ;;
    --skip-tmp)         SKIP_TMP=true; shift ;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^#\s\?//'
      exit 0
      ;;
    *) echo "FAIL: unknown arg '$1'"; exit 1 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$REPO_ROOT/target"
SKYCOT_OUTPUTS="$REPO_ROOT/.skycot/outputs"

# Whitelist check. Any path passed to delete_path MUST resolve
# under one of these prefixes. Anything else is REFUSED. This
# is the central safety primitive — every delete in this script
# routes through here.
is_whitelisted() {
  local path="$1"
  # Reject empty + reject paths that resolve to nothing.
  if [ -z "$path" ]; then return 1; fi
  # Reject suspicious traversal markers — defense-in-depth even
  # though realpath should resolve them.
  case "$path" in
    *..*) return 1 ;;
  esac
  case "$path" in
    "$TARGET_DIR"|"$TARGET_DIR"/*) return 0 ;;
    "$SKYCOT_OUTPUTS"/*) return 0 ;;
    /private/tmp/argo|/private/tmp/argo*) return 0 ;;
    /private/tmp/claude-*) return 0 ;;
    *) return 1 ;;
  esac
}

# Print or actually run the delete. Aborts the script on
# whitelist violation — there is no soft-fail path.
delete_path() {
  local path="$1"
  if [ -z "$path" ]; then
    echo "FAIL: empty path passed to delete_path"
    exit 1
  fi
  if ! is_whitelisted "$path"; then
    echo "FAIL: refused — '$path' is not under a whitelisted prefix"
    echo "      whitelist: target/, .skycot/outputs/*,"
    echo "                 /private/tmp/argo*, /private/tmp/claude-*"
    exit 1
  fi
  if [ ! -e "$path" ]; then
    return 0
  fi
  if $APPLY; then
    echo "  DELETE  $path"
    rm -rf -- "$path"
  else
    echo "  WOULD-DELETE  $path"
  fi
}

heading() {
  echo
  echo "=== $1 ==="
}

if $APPLY; then
  echo "*** APPLY MODE — paths below WILL be deleted. ***"
else
  echo "*** DRY-RUN — no changes. Pass --apply to delete. ***"
fi
echo "repo root:        $REPO_ROOT"
echo "max-age-days:     $MAX_AGE_DAYS"
echo "keep-recent:      $KEEP_RECENT"

# ── target/ ───────────────────────────────────────────────────
heading "target/ (cargo build cache)"
if $SKIP_TARGET; then
  echo "  SKIPPED — --skip-target"
elif [ -d "$TARGET_DIR" ]; then
  delete_path "$TARGET_DIR"
else
  echo "  (not present)"
fi

# ── .skycot/outputs/ ──────────────────────────────────────────
heading ".skycot/outputs/ (per-job worker bundles)"
if $SKIP_OUTPUTS; then
  echo "  SKIPPED — --skip-outputs"
elif [ -d "$SKYCOT_OUTPUTS" ]; then
  # Sort entries newest-first by mtime, mark the newest
  # KEEP_RECENT as protected, then delete anything older than
  # MAX_AGE_DAYS that isn't protected.
  cutoff_epoch=$(date -j -v "-${MAX_AGE_DAYS}d" +%s 2>/dev/null \
                  || date -d "${MAX_AGE_DAYS} days ago" +%s)
  if [ -z "$cutoff_epoch" ]; then
    echo "FAIL: could not compute cutoff date"
    exit 1
  fi

  # Build a sorted list of (mtime, path) pairs.
  entries=$(find "$SKYCOT_OUTPUTS" -mindepth 1 -maxdepth 1 -type d \
              -exec stat -f "%m %N" {} \; 2>/dev/null \
              | sort -nr)
  if [ -z "$entries" ]; then
    echo "  (no per-job bundles)"
  else
    keep_kept=0
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      mtime=$(echo "$line" | awk '{print $1}')
      path=$(echo "$line" | cut -d' ' -f2-)
      if [ "$keep_kept" -lt "$KEEP_RECENT" ]; then
        keep_kept=$((keep_kept + 1))
        echo "  KEEP (recent)  $path"
        continue
      fi
      if [ "$mtime" -ge "$cutoff_epoch" ]; then
        echo "  KEEP (within ${MAX_AGE_DAYS}d)  $path"
        continue
      fi
      delete_path "$path"
    done <<< "$entries"
  fi
else
  echo "  (not present)"
fi

# ── /private/tmp/argo* + /private/tmp/claude-* ────────────────
heading "/private/tmp/argo* + /private/tmp/claude-* (transient scratch)"
if $SKIP_TMP; then
  echo "  SKIPPED — --skip-tmp"
else
  any=false
  for entry in /private/tmp/argo /private/tmp/argo* /private/tmp/claude-*; do
    [ -e "$entry" ] || continue
    any=true
    delete_path "$entry"
  done
  if ! $any; then
    echo "  (no scratch dirs present)"
  fi
fi

echo
if $APPLY; then
  echo "✓ cleanup complete"
else
  echo "(dry-run finished — re-run with --apply to actually delete)"
fi
