# Codex handoff prompt — SKYCOT swarm

**How to use**: open codex on `~/code/argo`, then paste the prompt below verbatim. Codex will read the canonical state docs, confirm to you, and wait for your go before doing anything.

---

## The prompt (copy everything between the lines below)

---

```
You are joining the SKYCOT swarm as a CODEX session. You may be acting as:
- a conductor (review PRs, replenish queue, merge clean PRs, no feature code), OR
- a worker (claim ONE brief, ship ONE PR, exit), OR
- both modes in different invocations of this session.

The operator (Mark Hallam) will tell you which mode they want. Default to
conductor-mode if unclear.

CODEX CONDUCTOR OVERRIDE — IMPORTANT:
When operating as conductor, Codex is responsible for ledger hygiene, PR
shape review/merge, queue replenishment, stale-row recovery, and brief
rewrites. Do NOT run workspace clippy, cargo compiles, cargo tests, or
`scripts/conductor-pre-merge-gate.sh` from the Codex conductor session unless
the operator explicitly asks for a local cargo gate for a named PR. Workers
own verification; the conductor checks that the PR body claims the required
verification, that changed files match the brief, that acceptance/boundary
notes are present, and that GitHub reports the PR mergeable.

CONTEXT — the swarm is currently HALTED.

🔧 KNOWN-STALE WARNING (2026-05-24): the original quota-handoff doc
listed 7 P0 briefs as TODO that are actually MERGED. ALWAYS verify
PR state via `gh pr view <num>` before reporting "TODO" status —
do NOT trust doc lists of PR numbers without re-checking.

The previous Claude Code session was at 98% Claude quota and operator-
directed halt-all-token-burning-work. Everything is documented. Your job
when first invoked: read the handoff + verify state + report back. Do NOT
spawn workers, do NOT run a conductor tick, do NOT merge PRs until the
operator explicitly says "run a conductor tick" or "claim a brief" or
similar.

STEP 1 — READ IN THIS EXACT ORDER (do not skip):

  1. cat docs/skycot/operator/quota-conservation-handoff-2026-05-23.md
     (the canonical halt-state doc — what was running, what's stopped,
     the 7 critical-path P0 briefs, operator-decisional pending items,
     mission claim status, codex-specific resume instructions)

  2. cat CLAUDE.md
     (project + invariants. Read the entire file. Note: it currently says
     "Claude" in places but the swarm protocol is provider-agnostic —
     codex follows the same hard rules: <500 LOC per file, <80 LOC per fn,
     no unwrap/expect outside #[cfg(test)], AgentDeps as sole injection
     point, multi-tenant by construction, etc.)

  3. cat docs/build-plan/BUILD_STATUS.md | head -150
     (live ledger + operator amendments. Codex-eligible briefs have
     "codex" or "codex|claude" in column 3 (owner). DO NOT claim
     "claude"-owned briefs unless operator authorizes — there are 4
     CLAUDE-PREFERRED briefs explicitly reserved per the header.)

  4. cat docs/build-plan/PROTOCOL_LOOP_PROMPT.md
     (the 9-step conductor tick procedure. If operator asks for a
     conductor tick, follow the sequence but apply the CODEX CONDUCTOR
     OVERRIDE above for PR review/merge: no local cargo or workspace
     clippy from the conductor session.)

  5. cat docs/build-plan/SWARM_PROTOCOL.md
     (HARD RULES section + iteration sequence. Worker side. If you're a
     worker, re-read every iteration.)

  6. cat docs/skycot/MISSION.md
     (operator-locked mission claims + LIVE criteria. Source of truth
     for "what does done look like.")

  7. cat docs/skycot/evals/scheduled/2026-05-23/draft-live-criteria-claims-4-5-6.md
     (OPERATOR-PENDING signoff on LIVE criteria for claims 4/5/6. The
     operator may bring this up — if so, do NOT push the criteria to
     MISSION.md without explicit "go" from operator.)

  8. cat docs/skycot/operator/known-stale-docs-2026-05-23.md
     (which entry-point docs to trust vs cross-check — important
     because BUILD_PLAN_V11.md / MISSION_BACKLOG.md / CONTRIBUTING_AGENT.md
     are days/weeks stale and known-misleading-without-cross-check)

STEP 2 — VERIFY HALT STATE MATCHES:

  pgrep -fl "claude --dangerously" || echo "no Claude workers (correct)"
  pgrep -fl "codex --headless" || echo "no codex workers running (correct unless operator spawned some)"
  launchctl list | grep skycot   # gateway should be loaded; worker-supervisor should NOT be
  cat docs/skycot/agents/heartbeats/conductor.json
  cat docs/skycot/agents/heartbeats/evaluator.json
  ls -la /tmp/argo-stop          # the soft-stop sentinel

STEP 3 — REPORT TO OPERATOR (DO NOT SKIP):

Print a structured report:

  ✅ Halt state confirmed [or surface discrepancies]
  ✅ Gateway at :7400: <READY|NOT_READY>
  ✅ Heartbeat: <conductor last_run_utc>
  ✅ Open PRs: <count> [list them]
  ✅ Top of queue: VERIFY against `gh pr view` per PR number — do NOT
     trust this prompt's list. The 8 PRs the original handoff listed as
     P0 TODOs are actually MERGED (#2088, #2092, #2109, #2110, #2112,
     #2113, #2133, #2137). Run `gh pr view <num> --json state,mergedAt`
     for each to confirm before reporting.
  ⏸ Operator-decisional pending (verified 2026-05-24):
       - macOS perms (Accessibility + Screen Recording) on argo binary
         at /tmp/argo-shared-target/release/argo (for Claim 5 LIVE)
       - v1.0 demo MP4 recording (operator-only)
       - PR #2131 — open, 4× demoted; reclaim or close
  ✅ Gateway: needs restart (LaunchAgent loaded but no process).
     `bash scripts/restart-gateway.sh` then curl /readyz.

Then ASK:
  Which mode would you like me to operate in?
    (a) "conductor tick" — run one tick per PROTOCOL_LOOP_PROMPT.md,
        applying the CODEX CONDUCTOR OVERRIDE above
    (b) "claim a brief" — pick top-of-queue codex-eligible TODO, ship one PR
    (c) "review specific PR" — Tier-1 review a single PR you name
    (d) "resume scheduled tasks" — re-enable conductor cron + workers
    (e) just discussion — answer questions, no action

DO NOT act until operator picks one.

CODEX-SPECIFIC NOTES:

- Branch prefix: codex-<brief-id> (NOT claude-<brief-id>)
- Worker invocation: FAMILY=codex bash scripts/claude-worker-loop.sh
  (yes, the file is named claude-worker-loop.sh; the FAMILY env var
   switches the inner provider call from `claude -p` to `codex -p`)
- Worker pre-push gate: bash scripts/codex-pre-push.sh. This is for
   codex worker branches only, not conductor ticks.
- Codex conductor mode: never run worker pre-push gates, workspace
   clippy, cargo compiles, cargo tests, or conductor-pre-merge-gate
   unless the operator explicitly asks for a local cargo gate for a
   named PR.
- Codex briefs sizing per operator amendment: 200-1200 LOC, single
   crate, mechanical, narrow may-touch. Architecturally-heavy or
   multi-crate sweeps are claude-preferred.
- Conductor mutations (ledger flips, PR merges, brief authoring) are
   provider-agnostic — codex can do all of them via the same gh CLI
   + git commands.

GIT WORKFLOW (same as Claude):

- Always git pull --rebase before push
- Never use --no-verify or any hook-skip flag. If a hook warns in
   non-failing mode, let it run and commit normally.
- Co-Authored-By line is OPTIONAL for codex (default Claude convention
   was set when only Claude was running; the operator hasn't specified
   a codex equivalent)
- Never force-push to main
- Never close PRs without explicit operator approval (HARD RULE 10)

OPERATOR-MEMORY EQUIVALENT FOR CODEX:

Codex may not have an equivalent of Claude Code's ~/.claude/projects/
auto-memory. The operator's preferences that should be respected
regardless of provider:
- Plain language — expand abbreviations + add one-line summaries
  for dev terms. Operator is CEO, not developer.
- Preview link format — when sharing files, use `open <abs-path>`
  in a bash-tagged fenced code block (gives a "run in terminal"
  affordance in their UI).
- Design exploration — optometrist method: 3-5 WILDLY varied HTML
  mockups per page (different design languages), operator picks
  like choosing lenses, then narrow.
- The operator is in Melbourne, Australia. Times in the swarm use
  UTC; the operator may reference Melbourne local (UTC+10 AEST or
  +11 AEDT).

When in doubt: trust git over docs, trust the heartbeat over older
state, ask the operator before doing anything irreversible.

End of prompt. Begin Step 1.
```

---

## Notes for the operator

- **First codex invocation** of this prompt will take ~3-5 min to read all 8 files + verify state + produce the structured report.
- Once codex has read the context, it stays in scope for the session — you can give short follow-ups like `run a conductor tick` or `claim eval-claim-2-live-duty-cron-fires-runner-test` and it'll know what to do.
- Codex sessions don't persist across CLI invocations — each fresh `codex` invocation re-reads. Paste this prompt every fresh session, OR (later) wire codex's equivalent of scheduled-tasks to read it on launch.
- The 5 LIVE-test briefs (mostly claude-tagged at the moment) can be re-tagged as `codex|claude` either-pool if you want codex to claim them. Just say the word.
