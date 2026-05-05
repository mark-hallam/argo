#!/usr/bin/env bash
# Dev runtime safe-test gate.
#
# The "before autonomous coding" preflight. Runs:
#
#   1. Disk health check (./scripts/dev-safe-check.sh).
#   2. Free-space gate against the configured threshold.
#   3. cargo fmt --all -- --check.
#   4. cargo clippy --workspace --all-targets -- -D warnings.
#   5. cargo test --workspace.
#
# Each step's full output streams to stdout. The first step
# that fails aborts the rest; exit code is the failing step's
# exit code so CI can wire this directly.
#
# Threshold defaults to 50 GiB free; override via:
#
#   --threshold-gib N
#   ARGO_DEV_FREE_GIB=N
#
# Usage:
#   ./scripts/dev-safe-test.sh
#   ./scripts/dev-safe-test.sh --threshold-gib 80
#   ARGO_DEV_FREE_GIB=80 ./scripts/dev-safe-test.sh
#   ./scripts/dev-safe-test.sh --apply-cleanup-on-fail
#       (auto-runs `dev-safe-clean.sh --apply` if disk gate
#        fails — useful in long autonomous loops)
#
# Exit codes:
#   0   all green
#   1   disk gate failed
#   2   fmt failed
#   3   clippy failed
#   4   tests failed
#   5   prerequisite missing (cargo not on PATH)

set -uo pipefail

THRESHOLD_GIB="${ARGO_DEV_FREE_GIB:-50}"
APPLY_CLEAN_ON_FAIL=false

while [ $# -gt 0 ]; do
  case "$1" in
    --threshold-gib)            THRESHOLD_GIB="$2"; shift 2 ;;
    --apply-cleanup-on-fail)    APPLY_CLEAN_ON_FAIL=true; shift ;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^#\s\?//'
      exit 0
      ;;
    *) echo "FAIL: unknown arg '$1'"; exit 5 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT_DIR="$REPO_ROOT/scripts"

if ! command -v cargo >/dev/null 2>&1; then
  echo "FAIL: cargo not on PATH; source \$HOME/.cargo/env first"
  exit 5
fi

heading() {
  echo
  echo "=== $1 ==="
}

# ── 1. disk health snapshot ───────────────────────────────────
heading "1. disk health snapshot"
"$SCRIPT_DIR/dev-safe-check.sh"

# ── 2. free-space gate ────────────────────────────────────────
heading "2. free-space gate (>= ${THRESHOLD_GIB} GiB)"
df_line=$(df -Pk "$REPO_ROOT" | tail -1)
avail_kb=$(echo "$df_line" | awk '{print $4}')
threshold_kb=$((THRESHOLD_GIB * 1024 * 1024))
if [ "$avail_kb" -lt "$threshold_kb" ]; then
  avail_gib=$(awk -v k="$avail_kb" 'BEGIN{printf "%.1f", k/1024/1024}')
  echo "FAIL: only ${avail_gib} GiB free; need ${THRESHOLD_GIB} GiB"
  echo "      run ./scripts/dev-safe-clean.sh to see what can be reclaimed"
  if $APPLY_CLEAN_ON_FAIL; then
    echo "      --apply-cleanup-on-fail set — running cleanup now"
    "$SCRIPT_DIR/dev-safe-clean.sh" --apply
    avail_kb=$(df -Pk "$REPO_ROOT" | tail -1 | awk '{print $4}')
    if [ "$avail_kb" -lt "$threshold_kb" ]; then
      avail_gib=$(awk -v k="$avail_kb" 'BEGIN{printf "%.1f", k/1024/1024}')
      echo "FAIL: still only ${avail_gib} GiB free after cleanup"
      exit 1
    fi
    echo "      cleanup succeeded; continuing"
  else
    echo "      pass --apply-cleanup-on-fail to run cleanup automatically"
    exit 1
  fi
else
  avail_gib=$(awk -v k="$avail_kb" 'BEGIN{printf "%.1f", k/1024/1024}')
  echo "  OK — ${avail_gib} GiB free"
fi

# ── 3. cargo fmt --check ──────────────────────────────────────
heading "3. cargo fmt --all -- --check"
cd "$REPO_ROOT"
if ! cargo fmt --all -- --check; then
  echo "FAIL: rustfmt diff present; run \`cargo fmt --all\` to fix"
  exit 2
fi
echo "  OK — workspace formatted"

# ── 4. clippy ─────────────────────────────────────────────────
heading "4. cargo clippy --workspace --all-targets -- -D warnings"
if ! cargo clippy --workspace --all-targets -- -D warnings; then
  echo "FAIL: clippy is unhappy"
  exit 3
fi
echo "  OK — clippy clean"

# ── 5. workspace tests ────────────────────────────────────────
heading "5. cargo test --workspace"
if ! cargo test --workspace; then
  echo "FAIL: tests are red"
  exit 4
fi
echo "  OK — tests green"

echo
echo "✓ dev-safe-test PASSED"
