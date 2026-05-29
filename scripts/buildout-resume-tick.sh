#!/usr/bin/env bash
# Auto-resume tick for the autonomous platform build-out (operator-directed
# 2026-05-29). Fires hourly via com.skycot.buildout-resume.plist. It is a
# SAFETY NET: it only resumes the build when the primary conductor has gone
# quiet (e.g. hit a usage limit), so it never races an active conductor.
#
# Two guards:
#   1. mkdir-lock — only one resume tick runs at a time (atomic on macOS).
#   2. commit-age — skip if origin/main saw a conductor commit in the last
#      ~50 min (the primary conductor — interactive or a prior tick — is live).
#      50 (not lower) so it never races an interactive conductor mid-build
#      (some agent builds run 20-30 min with no commit). The mkdir-lock (held
#      across the whole claude -p run) guards against two ticks overlapping.
#      The plist checks every 30 min so absence is detected within ~30 min of
#      crossing the 50-min-quiet threshold.
#
# Mirrors the worker-loop's unattended invocation: `claude
# --dangerously-skip-permissions -p`, run from a login shell (the plist uses
# `/bin/zsh -lc`) so ~/.claude.json + login.keychain auth resolve.
#
# Operator off-switch: `touch ~/.argo/buildout-stop` (and unload the plist:
# `launchctl unload ~/Library/LaunchAgents/com.skycot.buildout-resume.plist`).
set -uo pipefail
REPO=~/code/argo
LOG="$HOME/.argo/logs/buildout-resume.log"
LOCK=/tmp/skycot-buildout-resume.lock
mkdir -p "$(dirname "$LOG")"
ts() { date '+%Y-%m-%dT%H:%M:%S%z'; }

cd "$REPO" 2>/dev/null || { echo "[$(ts)] repo missing — exit" >>"$LOG"; exit 0; }

# Off-switch.
if [ -f "$HOME/.argo/buildout-stop" ]; then
  echo "[$(ts)] buildout-stop present — exit." >>"$LOG"; exit 0
fi

# Guard 1: single tick at a time.
if ! mkdir "$LOCK" 2>/dev/null; then
  echo "[$(ts)] another resume tick holds the lock — skip." >>"$LOG"; exit 0
fi
trap 'rmdir "$LOCK" 2>/dev/null' EXIT

# Guard 2: don't race a live conductor.
git fetch origin -q 2>/dev/null || true
last=$(git log -1 --format=%ct origin/main 2>/dev/null || echo 0)
now=$(date +%s)
age_min=$(( (now - last) / 60 ))
if [ "$age_min" -lt 50 ]; then
  echo "[$(ts)] conductor active (last commit ${age_min}m ago) — skip." >>"$LOG"
  exit 0
fi

echo "[$(ts)] conductor quiet ${age_min}m — resuming build-out." >>"$LOG"
PROMPT='You are the SKYCOT autonomous platform build-out conductor, resuming after the primary session went quiet (likely a usage-limit reset). Read ~/code/argo/docs/skycot/overnight-build-out.md and follow its RESUME INSTRUCTIONS exactly: pick the highest-priority TODO or PARTIAL item that is NOT BLOCKED, build it, and verify it (for Rust: export CARGO_TARGET_DIR=$HOME/.cache/argo-shared-target CARGO_INCREMENTAL=0, then cargo test + clippy on the touched crate; for web: pnpm/npx vitest + tsc on touched files). git commit + push to origin/main with --no-verify ONLY when green. Append a timestamped Log line to that file each cycle. If an item needs operator input, mark it BLOCKED in the doc, commit that, and move to the next item. Keep building items until everything left is BLOCKED or budget runs low. HARD RULES: you are the SOLE Rust integrator on ONE shared target dir — do NOT create new target dirs (disk is tight); never break the running demo (gateway http://127.0.0.1:7400, SPA http://127.0.0.1:3000); NO THEATER (never mark DONE unless it is real and tested); no force-push (origin/main only, fast-forward); do not touch verticals/.genesis; commit small and often so the singleton guard stays accurate.'
timeout 7200 claude --dangerously-skip-permissions -p "$PROMPT" >>"$LOG" 2>&1
echo "[$(ts)] resume tick finished (exit $?)." >>"$LOG"
