#!/usr/bin/env bash
# check-brief-freshness.sh — advisory drift check for brief files
#
# Worker-side helper: before claiming a brief, run this against current
# main to detect common staleness patterns that have caused 4+ batches of
# may-touch widening rewrites this session.
#
# Output is ADVISORY (exit 0 with stderr warnings). Worker decides whether
# to claim, BLOCK, or escalate to conductor based on the report.
#
# Usage: scripts/check-brief-freshness.sh <brief-id>
# Example: scripts/check-brief-freshness.sh claude-worker-onboard-cli

set -uo pipefail  # NOT -e — keep going through all checks

BRIEF_ID="${1:-}"
if [[ -z "$BRIEF_ID" ]]; then
  echo "usage: $0 <brief-id>" >&2
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BRIEF="$REPO_ROOT/docs/build-plan/streams/$BRIEF_ID.md"

if [[ ! -f "$BRIEF" ]]; then
  echo "FAIL: brief file not found: $BRIEF" >&2
  exit 2
fi

DRIFT=0
note() { echo "  • $*" >&2; DRIFT=$((DRIFT+1)); }

echo "=== Brief freshness check: $BRIEF_ID ===" >&2

# 1. Validate may-touch files exist on current main
#    Extracts paths from lines like "- `crates/argo-cli/src/handlers/foo.rs`"
echo "[1/4] Validating may-touch file paths..." >&2
in_may_touch=0
while IFS= read -r line; do
  if [[ "$line" =~ ^"## Files you may touch" ]]; then in_may_touch=1; continue; fi
  if [[ "$line" =~ ^"## " && $in_may_touch -eq 1 ]]; then in_may_touch=0; fi
  if [[ $in_may_touch -eq 1 && "$line" =~ \`(crates/[^\`]+\.(rs|toml|sql))\` ]]; then
    path="${BASH_REMATCH[1]}"
    # Skip NEW-file markers
    if [[ "$line" =~ \(NEW ]]; then continue; fi
    if [[ ! -f "$REPO_ROOT/$path" ]]; then
      note "may-touch file missing on current main: $path"
    fi
  fi
done < "$BRIEF"

# 2. Verify "stub" / "not yet wired" / "is currently a stub" claims still hold
echo "[2/4] Validating stub/unimplemented claims..." >&2
grep -nE 'currently a stub|not yet wired|is a stub|returns Err.*not yet implemented' "$BRIEF" 2>/dev/null | while IFS=: read -r lineno text; do
  # Try to extract a path mentioned on the same brief line
  if [[ "$text" =~ \`([a-zA-Z_/.]+\.rs)\` ]]; then
    path="${BASH_REMATCH[1]}"
    full="$REPO_ROOT/$path"
    if [[ -f "$full" ]] && ! grep -qE 'not yet implemented|unimplemented!|todo!\(\)' "$full"; then
      note "brief claims '$path' is a stub but file has no stub markers (line $lineno of brief)"
    fi
  fi
done

# 3. Check for cli.rs at-500-LOC claim (common stale claim post-#968)
echo "[3/4] Checking cli.rs LOC claim staleness..." >&2
if grep -qE 'cli\.rs.*500 LOC' "$BRIEF"; then
  cli_lines=$(wc -l < "$REPO_ROOT/crates/argo-cli/src/cli.rs" 2>/dev/null || echo 0)
  if [[ $cli_lines -lt 100 ]]; then
    note "brief mentions cli.rs at 500 LOC but actual is $cli_lines (post-#968 split)"
  fi
fi

# 4. Worker-CLI brief pattern: should mention cli_worker_cmds.rs / cli_subcommands_skill.rs / handlers/worker_actions.rs
echo "[4/4] Checking CLI sibling-file coverage..." >&2
if grep -qE 'argo worker|WorkerCmd' "$BRIEF"; then
  for sibling in cli_worker_cmds.rs handlers/worker_actions.rs; do
    if ! grep -q "$sibling" "$BRIEF"; then
      note "worker-CLI brief is missing $sibling in may-touch (post-cli-split sibling)"
    fi
  done
fi
if grep -qE 'argo skill|SkillCmd' "$BRIEF"; then
  if ! grep -q 'cli_subcommands_skill.rs' "$BRIEF"; then
    note "skill-CLI brief is missing cli_subcommands_skill.rs in may-touch (post-cli-split sibling)"
  fi
fi

# Summary
echo "" >&2
if [[ $DRIFT -eq 0 ]]; then
  echo "✅ No drift signals — brief looks fresh against current main." >&2
  exit 0
else
  echo "⚠️  $DRIFT drift signal(s) detected — review before claiming." >&2
  echo "   Recommended: post a BUILD_STATUS.md BLOCKED note with the specific drift," >&2
  echo "   or escalate to conductor for may-touch widening." >&2
  exit 0  # ADVISORY — never fails CI
fi
