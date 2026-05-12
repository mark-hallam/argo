#!/usr/bin/env bash
# Refresh the committed CI baseline.
#
# Wraps `scripts/bench-record.sh` and copies the output to
# `scripts/bench-baselines/main.json` so the workflow at
# `.github/workflows/bench.yml.disabled` (.yml once billing is restored)
# has a stable reference to compare against. Operator runs after a
# deliberate perf-affecting change lands locally, then commits the
# refreshed baseline alongside the perf-changing PR.
#
# See `docs/skycot/runbook/perf-benches.md` → Regression detection.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

usage() {
  cat <<'EOF'
Usage: bench-baseline-refresh.sh [--skip-run] [--quiet] [--help]

Refresh scripts/bench-baselines/main.json from a fresh local bench run.

Options:
  --skip-run   Skip the run-bench.sh step (parse existing target/criterion/).
  --quiet      Redirect bench output to a log file.
  --help       Print this help.
EOF
}

ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --help|-h)           usage; exit 0 ;;
    --skip-run|--quiet)  ARGS+=("$1"); shift ;;
    *) echo "Unknown arg: $1" >&2; usage >&2; exit 2 ;;
  esac
done

DEST="scripts/bench-baselines/main.json"
TMP="$(mktemp -t bench-refresh.XXXXXX)"
trap 'rm -f "$TMP"' EXIT

bash scripts/bench-record.sh "${ARGS[@]}" --output "$TMP"

mkdir -p "$(dirname "$DEST")"
mv "$TMP" "$DEST"
trap - EXIT

echo "✓ refreshed $DEST"
echo "  Review the diff (\`git diff $DEST\`) and commit alongside the perf-changing PR."
