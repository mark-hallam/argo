#!/usr/bin/env bash
# ============================================================
# DEV ONLY — autonomous-coding-session safety net.
# NOT production tooling. Do not run on customer-facing
# SKYCOT deployments. Production cleanup goes through the
# operator runbook: docs/skycot/RUNBOOK.md.
# ============================================================
#
# Dev runtime disk-health check.
#
# Reports total / used / available disk for the volume backing
# the Argo repo, plus the size of the four known build/runtime
# artifact locations:
#
#   - target/                  (cargo build cache)
#   - .skycot/outputs/         (per-job worker output bundles)
#   - /private/tmp/argo*       (argo-side scratch tempdirs)
#   - /private/tmp/claude-*    (claude code runner tempfiles)
#
# Read-only. Never deletes or moves anything.
#
# Usage:
#   ./scripts/dev-safe-check.sh [--json]
#
# Exit codes:
#   0   success
#   2   prerequisite missing (df / du not on PATH)
#
# JSON mode emits one object — useful for piping into a
# preflight gate or a dashboard.

set -uo pipefail

JSON=false
for arg in "$@"; do
  case "$arg" in
    --json) JSON=true ;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^#\s\?//'
      exit 0
      ;;
    *) echo "FAIL: unknown arg '$arg'"; exit 1 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$REPO_ROOT/target"
SKYCOT_OUTPUTS="$REPO_ROOT/.skycot/outputs"

# Pull a `du -sk` byte-count for a path; returns 0 KB if the
# path is missing rather than failing — empty is a valid state.
size_kb() {
  local path="$1"
  if [ -e "$path" ]; then
    du -sk "$path" 2>/dev/null | awk '{print $1}'
  else
    echo 0
  fi
}

# Sum the sizes of every entry matching a glob — used for
# /private/tmp/argo* and /private/tmp/claude-* which expand to
# many sibling dirs.
size_kb_glob() {
  local glob="$1"
  local total=0
  # Use a subshell + nullglob behaviour so no-match expands to
  # nothing rather than the literal pattern.
  local expanded
  expanded=$(ls -d $glob 2>/dev/null || true)
  if [ -z "$expanded" ]; then
    echo 0
    return
  fi
  while IFS= read -r entry; do
    [ -z "$entry" ] && continue
    local kb
    kb=$(du -sk "$entry" 2>/dev/null | awk '{print $1}')
    total=$((total + kb))
  done <<< "$expanded"
  echo "$total"
}

human() {
  local kb="$1"
  if [ "$kb" -gt 1048576 ]; then
    awk -v k="$kb" 'BEGIN{printf "%.1f GiB", k/1024/1024}'
  elif [ "$kb" -gt 1024 ]; then
    awk -v k="$kb" 'BEGIN{printf "%.1f MiB", k/1024}'
  else
    echo "${kb} KiB"
  fi
}

# Disk numbers from df. Use POSIX -P + -k to get parseable
# 1k-block counts no matter the platform.
df_line=$(df -Pk "$REPO_ROOT" | tail -1)
df_total_kb=$(echo "$df_line" | awk '{print $2}')
df_used_kb=$(echo "$df_line" | awk '{print $3}')
df_avail_kb=$(echo "$df_line" | awk '{print $4}')

target_kb=$(size_kb "$TARGET_DIR")
outputs_kb=$(size_kb "$SKYCOT_OUTPUTS")
tmp_argo_kb=$(size_kb_glob "/private/tmp/argo*")
tmp_claude_kb=$(size_kb_glob "/private/tmp/claude-*")

if $JSON; then
  printf '{"repo_root":"%s","disk_total_kb":%d,"disk_used_kb":%d,"disk_avail_kb":%d,"target_kb":%d,"skycot_outputs_kb":%d,"tmp_argo_kb":%d,"tmp_claude_kb":%d}\n' \
    "$REPO_ROOT" "$df_total_kb" "$df_used_kb" "$df_avail_kb" \
    "$target_kb" "$outputs_kb" "$tmp_argo_kb" "$tmp_claude_kb"
  exit 0
fi

echo "Argo dev runtime disk health"
echo "============================"
echo "repo root:      $REPO_ROOT"
echo
echo "Volume containing the repo:"
printf "  total:         %s\n" "$(human "$df_total_kb")"
printf "  used:          %s\n" "$(human "$df_used_kb")"
printf "  available:     %s\n" "$(human "$df_avail_kb")"
echo
echo "Build + runtime artifacts:"
printf "  %-28s %s\n" "target/"                   "$(human "$target_kb")"
printf "  %-28s %s\n" ".skycot/outputs/"          "$(human "$outputs_kb")"
printf "  %-28s %s\n" "/private/tmp/argo*"        "$(human "$tmp_argo_kb")"
printf "  %-28s %s\n" "/private/tmp/claude-*"     "$(human "$tmp_claude_kb")"
echo
total_artifacts_kb=$((target_kb + outputs_kb + tmp_argo_kb + tmp_claude_kb))
echo "Reclaimable (sum of artifacts above):  $(human "$total_artifacts_kb")"
echo
echo "To clean: ./scripts/dev-safe-clean.sh           # dry-run"
echo "To apply: ./scripts/dev-safe-clean.sh --apply"
