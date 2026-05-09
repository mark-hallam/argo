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
echo "[1/9] Validating may-touch file paths..." >&2
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
echo "[2/9] Validating stub/unimplemented claims..." >&2
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
echo "[3/9] Checking cli.rs LOC claim staleness..." >&2
if grep -qE 'cli\.rs.*500 LOC' "$BRIEF"; then
  cli_lines=$(wc -l < "$REPO_ROOT/crates/argo-cli/src/cli.rs" 2>/dev/null || echo 0)
  if [[ $cli_lines -lt 100 ]]; then
    note "brief mentions cli.rs at 500 LOC but actual is $cli_lines (post-#968 split)"
  fi
fi

# 4. Worker-CLI brief pattern: should mention cli_worker_cmds.rs / cli_subcommands_skill.rs / handlers/worker_actions.rs
echo "[4/9] Checking CLI sibling-file coverage..." >&2
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

# 5. ArgoError variant existence check (added 2026-05-09 post-recurring-pattern).
#    Briefs frequently cite ArgoError variants that don't exist (e.g., NotFound,
#    Validation). Verify any cited variant is actually defined.
echo "[5/9] Checking ArgoError variant existence..." >&2
ARGO_ERROR_FILE="$REPO_ROOT/crates/argo-types/src/error.rs"
if [[ -f "$ARGO_ERROR_FILE" ]]; then
  grep -oE 'ArgoError::[A-Z][A-Za-z]+' "$BRIEF" 2>/dev/null \
    | sort -u | while read -r variant; do
        variant_name="${variant#ArgoError::}"
        if ! grep -qE "^\s+${variant_name}(\(|\s|,|$)" "$ARGO_ERROR_FILE"; then
          note "brief cites $variant but no such variant in argo-types/src/error.rs"
        fi
      done
fi

# 6. Migration slot collision (added 2026-05-09 post-v41/v42 near-miss).
#    If brief reserves a numbered migration slot, verify it's still free.
echo "[6/9] Checking migration-slot collision..." >&2
grep -oE '(migrations/|/v|slot )[0-9]{1,3}(__|\.rs|[^0-9])' "$BRIEF" 2>/dev/null \
  | grep -oE '[0-9]{1,3}' | sort -un | while read -r slot; do
      [[ "$slot" -lt 10 || "$slot" -gt 99 ]] && continue  # filter noise
      raw_match=$(ls "$REPO_ROOT"/crates/argo-store/migrations/${slot}__*.sql 2>/dev/null | head -1)
      ver_match=$(ls "$REPO_ROOT"/crates/argo-store/src/migrations/v${slot}.rs 2>/dev/null | head -1)
      if [[ -n "$raw_match" || -n "$ver_match" ]]; then
        # Brief references slot N — check if it's claiming N as NEW or referencing existing
        if grep -qE "(NEW migration|reserve|claim).*${slot}" "$BRIEF" \
           || grep -qE "${slot}__[a-z_]+\.sql.*NEW" "$BRIEF" \
           || grep -qE "v${slot}\.rs.*NEW" "$BRIEF"; then
          note "brief claims slot $slot as NEW but raw=$raw_match versioned=$ver_match already exists"
        fi
      fi
    done

# 7. May-touch LOC headroom (added 2026-05-09 post-#1066 turn.rs ceiling miss).
#    If any may-touch .rs file is >480 LOC on current main, the brief is at
#    high risk of pushing past the 500-LOC ceiling. Warn so brief-author can
#    add a split-prep brief first or worker can request widening.
echo "[7/9] Checking may-touch headroom (≤480 LOC headroom warning)..." >&2
in_may_touch=0
while IFS= read -r line; do
  if [[ "$line" =~ ^"## Files you may touch" ]]; then in_may_touch=1; continue; fi
  if [[ "$line" =~ ^"## " && $in_may_touch -eq 1 ]]; then in_may_touch=0; fi
  if [[ $in_may_touch -eq 1 && "$line" =~ \`(crates/[^\`]+\.rs)\` ]]; then
    path="${BASH_REMATCH[1]}"
    # Skip NEW-file markers
    if [[ "$line" =~ \(NEW ]]; then continue; fi
    if [[ -f "$REPO_ROOT/$path" ]]; then
      loc=$(wc -l < "$REPO_ROOT/$path" 2>/dev/null || echo 0)
      if [[ "$loc" -gt 480 ]]; then
        note "may-touch file at $loc LOC (≥480) is at-risk of 500-LOC ceiling: $path"
      fi
    fi
  fi
done < "$BRIEF"

# 8. Stub-vs-real-impl drift (added 2026-05-09 post-#1065 fs-executor finding).
#    Briefs say "X is currently a stub" but stubs become real impls when other
#    PRs land. Check more aggressively than the current spot-check.
echo "[8/9] Validating stub claims (deep)..." >&2
grep -nE 'is (currently )?a stub|stub (executor|handler|impl)|placeholder (executor|impl)' "$BRIEF" 2>/dev/null \
  | while IFS=: read -r lineno text; do
      # Extract any file path on the line
      while [[ "$text" =~ \`([a-zA-Z_/.]+\.rs)\` ]]; do
        path="${BASH_REMATCH[1]}"
        text="${text#*\`*\`}"
        full="$REPO_ROOT/$path"
        if [[ -f "$full" ]] && ! grep -qE 'unimplemented!|todo!\(\)|not yet implemented|placeholder' "$full"; then
          note "brief claims $path is a stub (line $lineno) but file has no stub markers"
        fi
      done
    done

# 9. Forward-only freshness anchor (Stream `claude-brief-freshness-header`).
#    New briefs carry `**Authored against**: origin/main <short-sha> on <YYYY-MM-DD>`.
#    Older briefs without the header skip this check silently.
echo "[9/9] Checking authored-against SHA distance (forward-only)..." >&2
authored_sha=$(grep -oE '\*\*Authored against\*\*:[[:space:]]*origin/main[[:space:]]+`?[a-f0-9]{7,40}`?' "$BRIEF" 2>/dev/null \
  | grep -oE '[a-f0-9]{7,40}' | head -1)
if [[ -n "$authored_sha" ]]; then
  if commits_behind=$(git -C "$REPO_ROOT" rev-list --count "$authored_sha..HEAD" 2>/dev/null); then
    if [[ "$commits_behind" -gt 100 ]]; then
      note "brief authored against $authored_sha is $commits_behind commits behind main (>100 — likely stale)"
    fi
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
