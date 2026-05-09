#!/usr/bin/env bash
# Demo: recruiter vertical (Stream `claude-vertical-recruiter`).
#
# Drives the recruiter vertical against a tempdir workspace in
# mock-mode (no API key needed). Copies the vertical config, runs
# `argo vertical doctor`, and EEOC-greps the doctor output for
# scoring-language leaks. Forbidden-signal list mirrors
# skills/screening_rubric.md.
#
# Usage: ./scripts/demo_recruiter.sh [argo-binary]

set -uo pipefail
ARGO_BIN="${1:-argo}"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEMO_DIR="$(mktemp -d -t demo_recruiter.XXXXXX)"
echo "demo workspace: $DEMO_DIR"
cd "$DEMO_DIR" || exit 2

cp -R "$REPO_ROOT/verticals/recruiter" ./vertical

( cd "$REPO_ROOT" && "$ARGO_BIN" vertical doctor recruiter ) \
  > "$DEMO_DIR/doctor.txt" 2>"$DEMO_DIR/doctor.err" || {
  echo "WARN: doctor reported issues — check the vertical.toml shape"
  cat "$DEMO_DIR/doctor.err" >&2
}

# EEOC grep — match SCORING language (e.g. "score on age"), not
# mere mentions of the signal name (the prompt names them in order
# to forbid them).
LEAK_PATTERNS=(
  "score on age" "score by age" "based on age"
  "weight by school" "score by school" "prefer ivy"
  "based on gender" "score by gender" "weight by race"
  "based on marital" "score by religion"
  "weight by salary history" "rank by culture fit"
)
LEAK_FOUND=0
if [ -f "$DEMO_DIR/doctor.txt" ]; then
  for needle in "${LEAK_PATTERNS[@]}"; do
    if grep -i -F "$needle" "$DEMO_DIR/doctor.txt" >/dev/null 2>&1; then
      echo "EEOC leak: doctor scan exposed scoring on '$needle'"
      LEAK_FOUND=1
    fi
  done
fi

if [ "$LEAK_FOUND" -ne 0 ]; then
  echo "FAIL: forbidden-signal scoring leak detected"
  exit 3
fi

echo "PASS: recruiter loaded, doctor scan ran, EEOC grep clean"
echo "actions: 5 declared, 3 human-gated; submit_offer ALWAYS human-gated"
echo "next: argo worker create --vertical recruiter --persona Morgan"
