#!/usr/bin/env bash
# worker-midrun-death-detect.sh — loop-side backstop for the one-shot
# "mid-run death" failure mode (brief fix-worker-oneshot-background-wait-midrun-death).
#
# THE FAILURE MODE
#   Worker sessions run as one-shot `claude -p` invocations. When the model
#   ends its turn "waiting for a notification" — a backgrounded cargo/clippy
#   task, or a `ScheduleWakeup` — the session simply TERMINATES; there is no
#   re-invocation in `-p` mode. The brief row stays CLAIMED forever, the
#   /tmp/<brief> worktree is deleted by the loop's cleanup, and a slot-iteration
#   is silently lost. Observed 3× on 2026-06-12 (SpendGate 65c327324 had to be
#   hand-salvaged; tier-quota died 10min in; a morning wave of 5 zombie rows).
#
#   The prompt-side hard rule in claude-worker-loop.sh tells the model never to
#   end its turn waiting; THIS is the un-forgettable backstop that self-heals
#   when the rule is missed.
#
# WHY A SEPARATE SOURCEABLE HELPER (vs. the old inline block)
#   The detector must be unit-testable in isolation (brief §3 requires bash
#   assertions over the three outcomes). claude-worker-loop.sh cannot be sourced
#   without running its `while true` loop, so the logic is extracted here as a
#   pure function, mirroring with-ledger-lock.sh / ledger-first-todo.sh /
#   stop-gate-enforce.sh. The loop sources this file and calls the function.
#
# ROOT-CAUSE FIX folded in: the prior inline ship-check matched the owner
#   column with `(claude|codex)` — which does NOT match the literal owner
#   string "claude or codex" that today's zombie rows carry. So the backstop
#   never even fired for them (the conductor had to release them by hand). This
#   detector is owner-AGNOSTIC (any non-empty owner), consistent with
#   SWARM_PROTOCOL.md HARD RULE 11 and the 2026-05-27 code-doc alignment that
#   already made the picker + claimer owner-agnostic.

# _wmd_shared_append <repo> <line>
#   Append a LOUD line to docs/skycot/agents/SHARED.md so the next conductor
#   tick's handoff scan sees the mid-run death. SHARED_MD_PATH override lets the
#   unit test redirect to a throwaway file (mirrors alert_claim_regex_reject in
#   claim-with-retry.sh).
_wmd_shared_append() {
  local repo="$1" line="$2"
  local shared="${SHARED_MD_PATH:-$repo/docs/skycot/agents/SHARED.md}"
  if [ -f "$shared" ]; then
    printf '\n%s\n' "$line" >> "$shared"
  fi
}

# _wmd_row_status <repo> <brief>
#   Echo the status (column 4) of the brief's ledger row, owner-agnostic.
#   Empty if the row is absent.
_wmd_row_status() {
  local repo="$1" brief="$2"
  awk -F'|' -v b="$brief" 'NF >= 5 {
    id = $2; gsub(/^[ \t]+|[ \t]+$/, "", id)
    if (id == b) {
      s = $4; gsub(/^[ \t]+|[ \t]+$/, "", s)
      print s; exit
    }
  }' "$repo/docs/build-plan/BUILD_STATUS.md" 2>/dev/null
}

# worker_midrun_death_check <brief> <branch> <repo>
#   Post-session backstop. Assumes the caller has already synced origin/main.
#   Outcomes:
#     - row no longer CLAIMED (IN_REVIEW / MERGED / TODO / BLOCKED): no-op,
#       return 0. The happy ship path is never touched.
#     - row still CLAIMED + branch has REAL work commits beyond origin/main:
#       push the branch to preserve the work (the SpendGate case) and append a
#       LOUD [worker-midrun-death] SHARED.md line; leave the row CLAIMED for the
#       conductor to salvage next tick.
#     - row still CLAIMED + no work commits: flip CLAIMED -> TODO with a
#       ♻️ AUTO-RELEASED (midrun death) note + append the same SHARED.md line.
#       The queue self-heals instead of starving the brief.
worker_midrun_death_check() {
  local brief="$1" branch="$2" repo="$3"
  local ts
  ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  local status
  status="$(_wmd_row_status "$repo" "$brief")"
  if [ "$status" != "CLAIMED" ]; then
    # Happy path (IN_REVIEW/MERGED) or already released — nothing to do.
    return 0
  fi

  # Make sure origin/main is current for the work-commit comparison.
  git -C "$repo" fetch origin main >/dev/null 2>&1 || true

  # Resolve the branch ref: local worktree branch first (its ref lives in the
  # main repo's ref db even after the worktree dir is removed), then a pushed
  # remote ref. Count commits the branch carries beyond origin/main; the
  # merge-base in `origin/main..<ref>` keeps the count correct even when main
  # advanced past the branch point.
  local ref="" tip="" work=0
  if git -C "$repo" rev-parse --verify --quiet "refs/heads/$branch" >/dev/null 2>&1; then
    ref="refs/heads/$branch"
  elif git -C "$repo" rev-parse --verify --quiet "refs/remotes/origin/$branch" >/dev/null 2>&1; then
    ref="refs/remotes/origin/$branch"
  fi
  if [ -n "$ref" ]; then
    work="$(git -C "$repo" rev-list --count "origin/main..$ref" 2>/dev/null || echo 0)"
    tip="$(git -C "$repo" rev-parse --short "$ref" 2>/dev/null || echo '')"
  fi

  if [ "${work:-0}" -gt 0 ]; then
    # PRESERVE: real work on the branch — push it so the conductor can salvage,
    # leave the row CLAIMED. Push is idempotent if the worker already pushed.
    echo "[$(date +%H:%M:%S)] 🪦 worker-midrun-death: $brief has $work work commit(s) on $branch (tip $tip); preserving — pushing branch, leaving row CLAIMED for conductor salvage."
    git -C "$repo" push -u origin "$branch" 2>&1 | tail -1 || true
    _wmd_shared_append "$repo" "- $ts [worker-midrun-death] brief '$brief' — one-shot worker session ended mid-run holding REAL work ($work commit(s), branch '$branch' tip $tip). Branch PUSHED to preserve it; row left CLAIMED. CONDUCTOR: salvage the branch (rebase + open PR or finish), like SpendGate #2865. Root: a one-shot \`claude -p\` session ended its turn to 'wait' for a background task — see streams/fix-worker-oneshot-background-wait-midrun-death.md."
    return 0
  fi

  # AUTO-RELEASE: no work on the branch — flip CLAIMED -> TODO so the queue
  # self-heals. Owner-agnostic regex (any non-empty owner) — this is the bug
  # the prior `(claude|codex)`-only match could not handle for "claude or codex"
  # rows. The note is appended INSIDE the last cell (capture stops before the
  # trailing ` |`) so the row format stays valid for claim-with-retry.
  echo "[$(date +%H:%M:%S)] ♻️  worker-midrun-death: $brief still CLAIMED with no work commits; auto-releasing to TODO."
  AUTO_RELEASE_NOTE=" **♻️ AUTO-RELEASED (midrun death) $ts by worker-loop:** one-shot session ended its turn to 'wait' for a background task and terminated before opening a PR; no work commits on branch '$branch'. Re-pick cleanly. See streams/fix-worker-oneshot-background-wait-midrun-death.md." \
  python3 - "$brief" "$repo" <<'PY'
import os, re, sys, pathlib
brief, repo = sys.argv[1:3]
p = pathlib.Path(repo) / "docs/build-plan/BUILD_STATUS.md"
text = p.read_text()
note = os.environ["AUTO_RELEASE_NOTE"]
new, n = re.subn(
    rf"^(\| {re.escape(brief)} \| [^|]+) \| CLAIMED \|(.*) \|$",
    lambda m: f"{m.group(1)} | TODO |{m.group(2)}{note} |",
    text, count=1, flags=re.MULTILINE,
)
if n:
    p.write_text(new)
PY

  # Commit + push the release with a small bounded rebase-retry. Skip if the
  # edit was a no-op (row changed under us between status read and edit).
  git -C "$repo" add docs/build-plan/BUILD_STATUS.md
  if git -C "$repo" diff --cached --quiet; then
    echo "[$(date +%H:%M:%S)] worker-midrun-death: row no longer CLAIMED after re-read; nothing to release."
  else
    local _a _out _rc
    for _a in 1 2 3; do
      git -C "$repo" commit -m "status: auto-release $brief (worker midrun death; no PR opened)" >/dev/null 2>&1 || true
      git -C "$repo" fetch origin main >/dev/null 2>&1 || true
      git -C "$repo" pull --rebase origin main >/dev/null 2>&1 || true
      _out="$(git -C "$repo" push origin main 2>&1)"; _rc=$?
      if [ "$_rc" -eq 0 ]; then break; fi
      echo "[$(date +%H:%M:%S)] worker-midrun-death: ledger push attempt $_a/3 lost the race; retrying..."
      sleep 1
    done
  fi

  _wmd_shared_append "$repo" "- $ts [worker-midrun-death] brief '$brief' — one-shot worker session ended mid-run with NO work commits (claim-commit only, branch '$branch'). AUTO-RELEASED CLAIMED -> TODO; the queue self-heals. Root: the session ended its turn to 'wait' for a background task — see streams/fix-worker-oneshot-background-wait-midrun-death.md."
  return 0
}

# CLI entrypoint when executed directly (sourcing just defines the functions).
if [ "${BASH_SOURCE[0]:-$0}" = "${0}" ]; then
  if [ "$#" -lt 3 ]; then
    echo "usage: worker-midrun-death-detect.sh <brief> <branch> <repo>" >&2
    exit 2
  fi
  worker_midrun_death_check "$1" "$2" "$3"
fi
