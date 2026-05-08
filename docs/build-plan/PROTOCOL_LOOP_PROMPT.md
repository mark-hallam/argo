# PROTOCOL_LOOP_PROMPT — the conductor-tick prompt

Paste the prompt below into `/loop 5m <prompt>` on the project repo.
This drives the conductor tab. See `PROTOCOL.md` for the full
architecture and `CONTRIBUTING_AGENT.md` for the worker-side rules.

How to run:

1. Substitute the four placeholders in angle brackets:
   - `<REPO_PATH>` — absolute path to the repo (e.g.
     `/Users/me/code/myproject`).
   - `<PLAN_DIR>` — relative path of the plan directory inside the
     repo (e.g. `docs/build-plan`).
   - `<WORKER_BRANCH_PREFIXES>` — comma-list of branch prefixes the
     swarm uses (e.g. `claude-,codex-`).
   - `<PROJECT_NICKNAME>` — what to call the swarm in the report
     (e.g. `SKYCOT 14-day v1`).
2. Open ONE conductor session on `<REPO_PATH>`. Any coding platform may
   run it if it can execute this tick reliably. The role is identical:
   ledger, PR review, queue replenishment, brief rewrites; no feature
   code.
3. Run the prompt below through your platform's loop/automation surface
   or execute the same sequence manually without changing the repository
   protocol.
4. Cancel the loop when every brief in BUILD_STATUS.md is MERGED
   AND the v1 demo gate is green, or by Ctrl-C / human intervention.

The lines flagged `[PROJECT-SPECIFIC]` are the ones you'll change
per project. Every other line is reusable verbatim.

**Note on step 8 (queue replenishment)**: the prompt differentiates
codex vs claude brief sizing. Codex briefs are smaller (200–1200
LOC), single-crate, mechanical, narrow may-touch. Claude briefs are
larger (300–2000 LOC), can span crates, can make architectural
decisions, can include integration tests. When the conductor
brainstorms new briefs at step 8, it picks the sizing flavor based
on which queue is short.

**Post-v1 idea sources for step 8**: once the v1 demo gate is green,
queue replenishment draws from these sources, in priority order:

1. **`MISSION_BACKLOG.md`** — the canonical 280-item backlog from
   the tick-198 deep-research pass. Each item is status-tagged.
   Pick items tagged `[NEW]` (no brief exists yet); skip `[BRIEF]`
   (already authored), `[SHIPPED]` (merged), `[DEFER]` (not yet
   actionable). When authoring a brief from this source, name it
   `mission-<short-slug>.md` so the provenance is visible.
2. `BUILD_PLAN_V11.md` — the phased plan from v1.1 production
   hardening through v2.2 demo verticals. Use for items not yet
   in MISSION_BACKLOG (rare; backlog is the superset).
3. Recent PR-body "deferred to follow-up" sections — drains as
   the queue moves.
4. `MISSION.md` gaps — last-resort source; the substrate-vs-runtime
   section there points at the same gap MISSION_BACKLOG enumerates.
5. `git log -100` — for items implied by recent merges.

The conductor picks 3–5 items per replenishment, weighted by which
queue is short, and updates the picked items' status tag in
MISSION_BACKLOG.md from `[NEW]` to `[BRIEF: <new-id>]` in the same
commit that adds the brief file.

---

```
5m Conductor tick for the <PROJECT_NICKNAME> swarm. Run this exact sequence:

0. **MANDATORY QUEUE-DEPTH CHECK FIRST.** Before reading PRs or doing any merge work:
   codex_todo = `grep -cE '^\| .* \| codex \| TODO ' <PLAN_DIR>/BUILD_STATUS.md`
   claude_todo = `grep -cE '^\| .* \| claude \| TODO ' <PLAN_DIR>/BUILD_STATUS.md`
   If EITHER is ≤8, fire Step 8 (queue replenishment) NOW before continuing to Step 1.
   This rule is load-bearing — workers stand down if queue runs dry. The conductor must
   not skip this check, even when "nothing else looks pressing."
   If this tab previously acted as a worker and has a CLAIMED row or feature-code WIP,
   cleanly release/finish that worker state before doing conductor mutations. Never mix
   worker implementation with conductor review/replenishment in the same tick.

1. cd <REPO_PATH> && git checkout main && git pull --ff-only origin main 2>&1 | tail -3

2. git log --oneline origin/main -8  (see what landed since last tick)

3. gh pr list --state open --json number,title,headRefName,author,createdAt 2>&1  (see open PRs)

4. cat <PLAN_DIR>/BUILD_STATUS.md | head -60  (see queue state)

5. For any NEW open PR from a <WORKER_BRANCH_PREFIXES> branch I haven't reviewed yet:
   - gh pr diff <num> --color never | head -300 (skim the diff)
   - Verify against the brief's acceptance checklist — **TIERED VERIFICATION (tick368 amendment)**:
     - **Tier 1 (default — ~1-2min)**: diff-shape review only. Confirm: (a) files match brief's may-touch list, (b) every modified file <500 LOC, (c) all acceptance boxes ticked in PR body, (d) any boundary deviations declared, (e) PR body explicitly claims crate-scoped clippy/tests plus the final `clippy --workspace` sanity check from SWARM_PROTOCOL.md (Verification section), (f) `gh pr view --json mergeStateStatus` returns MERGEABLE not DIRTY. **Trust the worker's verification claim.** Workers own the cargo verification; the conductor re-running it duplicates 9+ minutes of work that already passed.
     - **Tier 2 (~2min)**: targeted single-crate clippy ONLY when the diff touches a schema migration, multi-crate refactor, or any `argo-store/migrations/` file. Run `cargo clippy -p <touched-crate> --all-targets -- -D warnings` (NOT --workspace). 1-2min instead of 9.
     - **Tier 3 (~10min, full local verify)**: only when Tier 1 surfaces a red flag: missing PR body, no acceptance ticks, out-of-may-touch edits, brand-new crate added, security-sensitive code, or worker history of broken submissions. Spin worktree + `cargo clippy --workspace`.
   - If clean: gh pr merge <num> --squash --delete-branch, then update BUILD_STATUS.md row to MERGED with PR num, commit "status: <brief-id> merged (#<num>)", push to main
   - If issues: gh pr comment <num> --body "<specific issue>", note in BUILD_STATUS.md
   - Track which PRs I've already reviewed across ticks (use git log to see my own merge commits)

6. Do not write feature code from the conductor tab. If there is process/doc work already
   in progress for the conductor role, continue it; otherwise skip to BLOCKED triage.

7. If a brief has been BLOCKED, investigate the reason and decide: expand its may-touch list (small fix in BUILD_STATUS.md + brief), or split into sub-briefs, or escalate to the human operator.
   Also sweep active manager packages under `<PLAN_DIR>/work-packages/`: parent row still CLAIMED, package heartbeat fresh, `max_depth: 1`, child count + spawn_requests within spawn budget, and child allowed files inside the parent brief's may-touch list. If a package heartbeat is stale (>30m warn, >60m consider reclaim, >90m reclaim unless human overrides), comment or update the parent row instead of adding child rows to BUILD_STATUS.md.

8. Queue replenishment:
   a. codex_todo = grep -c "| codex |.*| TODO |" <PLAN_DIR>/BUILD_STATUS.md
      claude_todo = grep -c "| claude |.*| TODO |" <PLAN_DIR>/BUILD_STATUS.md
   b. If codex_todo <= 8: brainstorm 3-5 NEW codex-sized briefs. Idea sources, in priority
      order: (1) `<PLAN_DIR>/MISSION_BACKLOG.md` items tagged `[NEW]` — name authored briefs
      `mission-<slug>` so the source is traceable; in the same commit, flip the picked
      backlog items' status tag from `[NEW]` to `[BRIEF: mission-<slug>]`. (2) BUILD_PLAN_V11.md
      phased plan (the v1.x / v2.x stage that's currently active). (3) recent PR-body
      "deferred to follow-up" sections. (4) MISSION.md gaps. (5) git log -100. Each brief
      follows PROTOCOL_BRIEF_TEMPLATE.md. Sizing: 200-1200 LOC, narrow may-touch,
      single-crate focus.
   c. If claude_todo <= 8: same for claude. Sizing: 300-2000 LOC, multi-crate OK,
      architectural decisions OK, integration tests welcome. Same `mission-<slug>` naming
      + backlog-tag-flip discipline applies.
   d. **Brief authoring discipline (tick-58 lesson)**: before publishing a brief, ground
      the may-touch list in the real codebase. Run `find crates -name '*.rs' | grep <thing>`
      or `rg -l <symbol>` to verify every file path exists. Imagined paths cause BLOCKED
      claims — 30 seconds of verification beats six dead briefs.
      Also stamp every NEW brief with the freshness header from
      `PROTOCOL_BRIEF_TEMPLATE.md` (Stream `claude-brief-freshness-header`):
      `**Authored against**: origin/main <short-sha> on <YYYY-MM-DD>`. Capture
      via `git rev-parse --short origin/main` at author-time. Forward-only —
      do NOT retrofit older briefs.
   e. Push ONE commit per replenishment: "briefs: replenish <owner> queue (<id1>, <id2>, ...)".
      If sourced from MISSION_BACKLOG, include the backlog-tag-flip in the SAME commit
      (so MISSION_BACKLOG and the new brief files land atomically — never re-author the
      same item across ticks).
   f. Skip if both above 8.

9. If everything is in flight and there's nothing to do, report back briefly with
   "swarm healthy, X PRs in review, Y briefs claimed, Z merged, codex_todo=A, claude_todo=B"
   and end the tick.

Do not stop the loop. Cancel only when the user asks to stop OR when every brief in BUILD_STATUS.md is MERGED AND the v1 demo gate is green.
```

---

## Notes

- The 5-minute interval is a default. For larger swarms (8+ workers)
  drop to 3 minutes. For sleepier projects (1–2 workers, hand-paced)
  raise to 10 minutes or run unscheduled.
- `head -8` on the git log and `head -300` on the diff are tuned for
  Claude's context budget per tick. Bump if you want deeper review.
- The conductor reports "nothing to do" cleanly — that's the
  expected state on most ticks. Don't fight it.
- If the conductor tab itself crashes / loses state, restart the
  loop in a fresh tab. The ledger on origin/main IS the state; the
  conductor's local memory is disposable.
- Step 8 (queue replenishment) is what keeps the swarm fed
  indefinitely. Without it, the loop runs out of work as soon as
  the seed briefs all merge. The trigger is simple: count TODO
  rows per owner, fire if either is ≤8, write 3–5 new briefs in
  one commit. See `PROTOCOL.md` "Queue replenishment" for the
  idea sources, sizing rules, and stop condition.
- **MISSION_BACKLOG.md is the canonical source** of un-shipped
  work post-tick-198. It carries ~280 status-tagged items
  ([NEW] / [BRIEF: <id>] / [SHIPPED] / [PARTIAL] / [DEFER]).
  Conductor's job at Step 8 is to pick `[NEW]` items, author
  briefs named `mission-<slug>`, and flip the backlog tag in the
  SAME replenishment commit. NEVER author a brief from a
  `[BRIEF: ...]` item (it already exists; re-authoring causes
  BLOCKED claims). NEVER author from a `[SHIPPED]` item (it's
  merged on main; re-authoring is the auto-compaction-amnesia trap
  documented in BUILD_STATUS.md tick-115 amendment).
- Hard rules every tick (carry over from earlier protocol revisions):
  always run from `<REPO_PATH>` on the main branch after a fresh
  `git pull --ff-only`; never `git commit -am` (always stage explicit
  files); never force-push to main; never modify a worker's branch
  except via squash-merge or close.
