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
4. Cancel the loop when the v1.0 ship gate is met:
   `docs/skycot/demos/v1-mission-mvp.mp4` exists AND
   `./scripts/smoke-skycot.sh` passes including BrowserExecutor +
   Slack-approval scenarios (per BUILD_STATUS.md header). Or by
   Ctrl-C / human intervention.

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
4. `docs/skycot/MISSION.md` gaps — last-resort source; the substrate-vs-runtime
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
   If EITHER is ≤25 (target depth 30-40), fire Step 8 (queue replenishment) NOW before
   continuing to Step 1. (threshold per BUILD_STATUS.md operator amendment 2026-05-10;
   verify the live header at run time — operator amendments override this template.)
   This rule is load-bearing — workers stand down if queue runs dry. The conductor must
   not skip this check, even when "nothing else looks pressing."
   If this tab previously acted as a worker and has a CLAIMED row or feature-code WIP,
   cleanly release/finish that worker state before doing conductor mutations. Never mix
   worker implementation with conductor review/replenishment in the same tick.
   Do not perform any Step 8 ledger/brief mutation until Step 1's singleton preflight
   has passed.

1. cd <REPO_PATH> && git checkout main && git pull --ff-only origin main 2>&1 | tail -3
   Then run TWO preflight checks before the first conductor mutation (queue
   replenishment, BUILD_STATUS flip, merge, or heartbeat write):

   (a) `bash scripts/check-conductor-singleton.sh; SINGLETON_RC=$?` — checks
       the heartbeat file. Exit 0 (`missing`/`stale`) means proceed. Exit 1
       with `alive` or `newer` means another conductor is live/newer: stop
       and surface to the operator. Exit 1 with `invalid` means fix or
       escalate the heartbeat parse state before mutating.

   (b) `bash scripts/check-recent-conductor-commit.sh; RECENT_RC=$?` —
       checks `git log` on `origin/main` for a peer conductor commit in
       the last 10 minutes. Exit 0 means no recent peer commit; proceed.
       Exit 1 means another conductor JUST landed work; **short-circuit
       this tick** to avoid colliding mid-edit (the worker-supervisor
       watchdog stashes mid-edit trees every ~2 min, which can orphan
       untracked brief files from their ledger rows when two conductor
       sessions overlap). Heartbeat refresh + read-only status report
       are still OK on short-circuit; merge/replenishment/brief-author
       mutations are not. Lesson added 2026-05-25 after a cross-conductor
       near-miss in the T07-40/T09-15 session.

2. git log --oneline origin/main -8  (see what landed since last tick)

3. gh pr list --state open --json number,title,headRefName,author,createdAt 2>&1  (see open PRs)

4. cat <PLAN_DIR>/BUILD_STATUS.md | head -60  (see queue state)

4.5. **Scheduled-auditor report check (operator amendment 2026-05-21).** `cat docs/skycot/audit/scheduled/LATEST.md` if the file exists. If P0 recommendations are present AND the report is ≤12h old, surface them at the top of THIS tick — action what's tickable, queue what needs a brief (Step 8), defer what needs operator input. If the report is >12h old OR no LATEST.md exists, proceed normally. Auditor charter: `docs/skycot/audit/AUDITOR_CHARTER.md`. The auditor is read-only; this step only consumes its output.

4.5b. **Unread cross-agent handoffs check (operator amendment 2026-05-23).** `bash scripts/check-unread-handoffs.sh; UNREAD_RC=$?`. Exit codes: `0`=clean OR only already-landed/operator-only/informational handoffs remain and the script printed the `ACK PATH`; `1`=BLOCK (≥1 unread handoff still needs queue/ledger/operator handling), `2`=parse error (proceed cautiously). On exit 1: the script prints each unread entry — for EACH entry decide: (a) author a brief (Step 8), (b) flip a ledger row, (c) surface to operator in the tick summary. Use `bash scripts/classify-unread-handoffs.sh` as the read-only follow-on helper when the unread list mixes already-landed, queue-needed, operator-only, and informational handoffs. The tick MUST NOT declare complete until every unread entry is actioned or explicitly noted as "informational, no action." Once actioned, Step 4.6's heartbeat refresh marks them read. This prevents the failure mode where the evaluator (or other agent) writes a handoff to `docs/skycot/agents/SHARED.md` and the conductor's tick walks past it.

4.6. **Agent-liveness watchdog (operator amendment 2026-05-21).** `bash scripts/check-agent-heartbeats.sh`. If any non-conductor agent is reported STALE (🔴), surface it in the tick summary at the end ("auditor stale since YYYY-MM-DDTHH-MM — operator should re-launch via the prompt in `docs/skycot/agents/AGENTS_OVERVIEW.md`"). Don't try to restart agents from the conductor — only operator can re-launch their sessions. Then write the conductor's own heartbeat: update `docs/skycot/agents/heartbeats/conductor.json` with current UTC timestamp (use `date -u +"%Y-%m-%dT%H:%M:%SZ"` — the `-u` flag is load-bearing; see `docs/skycot/agents/heartbeats/WRITE_SIDE_RUNBOOK.md` for the past wrong-time race this prevents), `expected_cadence_seconds: 3600`, this tick's session label, and a 1-line run note. Append one line to `docs/skycot/agents/SHARED.md`: `<UTC-ISO> [conductor] <one-line this-tick-did>`. The heartbeat refresh happens after all Step 4.5b handoffs are either actioned or explicitly acknowledged by the `ACK PATH`; operator-only and informational entries are considered read once they are surfaced/summarized, so they must not freeze `last_run_utc` indefinitely. If `scripts/check-unread-handoffs.sh` warns `conductor cutoff staleness`, still finish the tick and refresh the heartbeat after resolving/acknowledging the printed entries. When authoring or editing any new scheduled-task `SKILL.md`, apply the same `-u` rule — the WRITE_SIDE_RUNBOOK has the one-liner sed audit.

4.7. **Gateway-freshness drift check (operator amendment 2026-05-23).** `bash scripts/check-gateway-freshness.sh > /tmp/gateway-freshness.txt 2>&1; FRESH_RC=$?`. Read the exit code: `0`=FRESH (skip), `1`=STALE, `2`=UNREACHABLE, `3`=PARSE_ERROR. For exit 1, prepend a `🟡 GATEWAY-STALE: $(cat /tmp/gateway-freshness.txt | head -1)` line to the tick summary so the operator sees the drift and can decide when to `make restart-live-gateway`. For exit 2 or 3, surface the same line with `🟡 GATEWAY-UNREACHABLE` / `🟡 GATEWAY-FRESHNESS-PARSE-ERROR` respectively. Read-only / non-blocking — the conductor only reports; the operator restarts.

4.8. **MERGED-FINDINGS-ONLY fix-by sweep (operator amendment 2026-05-23).** `bash scripts/check-findings-only-fix-by.sh > /tmp/findings-fix-by.txt 2>&1; FIX_BY_RC=$?`. Read exit code: `0`=no overdue rows (skip), `1`=≥1 overdue row found. For exit 1, prepend the 🔴 lines from `/tmp/findings-fix-by.txt` to the tick summary so the next agent/operator sees which `MERGED-FINDINGS-ONLY` deadlines have lapsed without a fix-PR. Read-only / non-blocking — the conductor surfaces the warning; the operator decides whether to author a fix-brief, extend the budget, or downgrade severity. The wire format is documented in `PROTOCOL.md` under "MERGED-FINDINGS-ONLY contract".

4.9. **Pre-merge P0 aging sweep (operator amendments 2026-05-23, closes audit 2026-05-23T09-40 §4 P1 #4 + evaluator round-30 cu-macos-blocker handoff).** `bash scripts/check-todo-aging.sh > /tmp/todo-aging.txt 2>&1`. Walks `BUILD_STATUS.md` for rows where `status` is one of `TODO` / `CLAIMED` / `CHANGES-REQUESTED` AND the note is tagged P0 (🔴 P0 / 🚨 P0 / **P0 / P0: / OPERATOR-LOCKED). Classifies each by age since the most-recent ISO timestamp (or date-only NEW token) in the row note: 🟢 <24h within budget, 🟠 24-48h aging, 🔴 ≥48h overdue. For any 🟠 or 🔴 lines, prepend them to the tick summary so the next agent/operator sees which P0 rows have been stuck too long. Read-only / non-blocking — exits 0 by design (informational only). Initial TODO-only landing via #2134 (ops-add-todo-aging-gate-script); broadened to CLAIMED + CHANGES-REQUESTED via ops-fix-by-tracker-add-p0-cu-macos-blocker so an actively-claimed-but-not-shipping P0 (like the cu-macos #2074 regression chain) also escalates. 4.8's job is MERGED-FINDINGS-ONLY rows; this sweep covers everything pre-merge.

5. For any NEW open PR from a <WORKER_BRANCH_PREFIXES> branch I haven't reviewed yet:
   - Optional overlap scan: `bash scripts/open-pr-overlap-triage.sh` to surface open PRs touching the same non-doc file path before choosing merge order.
   - gh pr diff <num> --color never | head -300 (skim the diff)
   - Verify against the brief's acceptance checklist — **TIERED VERIFICATION (tick368 amendment)**:
     - **Tier 1 (default — ~1-2min)**: diff-shape review only. Confirm: (a) files match brief's may-touch list, (b) every modified file <500 LOC, (c) all acceptance boxes ticked in PR body, (d) any boundary deviations declared, (e) PR body explicitly claims crate-scoped clippy/tests plus the final `clippy --workspace` sanity check from SWARM_PROTOCOL.md (Verification section), (f) `gh pr view --json mergeStateStatus` returns MERGEABLE not DIRTY. **Trust the worker's verification claim.** Workers own the cargo verification; the conductor re-running it duplicates 9+ minutes of work that already passed.
     - **Tier 2 (~2min)**: targeted single-crate clippy ONLY when the diff touches a schema migration, multi-crate refactor, or any `argo-store/migrations/` file. Run `cargo clippy -p <touched-crate> --all-targets -- -D warnings` (NOT --workspace). 1-2min instead of 9.
     - **Tier 3 (~10min, full local verify)**: only when Tier 1 surfaces a red flag: missing PR body, no acceptance ticks, out-of-may-touch edits, brand-new crate added, security-sensitive code, or worker history of broken submissions. Spin worktree + `cargo clippy --workspace`.
   - If clean: **run `bash scripts/conductor-pre-merge-gate.sh` BEFORE the admin-merge** (2026-05-18 amendment: while GHA is operator-disabled, this script reproduces the workspace-clippy + LOC budget + migration-slot + parallel-pub-types gates locally). Then `gh pr merge <num> --squash --delete-branch --admin`, run `bash scripts/auto-restart-gateway-on-schema-bump.sh "$(gh pr view <num> --json mergeCommit --jq '.mergeCommit.oid')"`, then update BUILD_STATUS.md row to MERGED with PR num, commit "status: <brief-id> merged (#<num>)", push to main. If the pre-merge gate FAILS: do NOT merge; comment on the PR with the failing gate output, demote the row to TODO with a fix-request note, and ping the next conductor.
   - If issues: gh pr comment <num> --body "<specific issue>", note in BUILD_STATUS.md
   - **Stale generated doc-tree fix-request template.** Use this when a
     worker PR accidentally carries stale generated auditor/evaluator/
     heartbeat docs that are outside the brief. First identify the exact
     stale paths from `gh pr diff`; do not include legitimate documentation
     files the brief explicitly edits. Copy-ready comment body:
     "This PR includes stale generated doc-tree changes outside the brief.
     Please restore only those generated paths from `origin/main`, keep your
     intended code/docs edits, then re-run verification:
     `git fetch origin main`
     `git checkout origin/main -- docs/skycot/audit/scheduled/LATEST.md docs/skycot/evals/scheduled/<stale-file>.md docs/skycot/agents/heartbeats/<stale-agent>.json docs/skycot/agents/SHARED.md`
     `git status --short`
     Replace the example path list with the exact stale paths in this PR.
     Do not run `git checkout origin/main -- .`, `git reset --hard`, or broad
     glob restores; those can erase the worker's intended changes."
   - Track which PRs I've already reviewed across ticks (use git log to see my own merge commits)
   - **Stale-IN_REVIEW sweep (tick465 amendment).** A worker that opens a PR, gets a fix-request comment, then exits without re-pushing leaves the PR + ledger row stuck IN_REVIEW indefinitely. After handling NEW PRs above, sweep stuck ones:
     ```
     gh pr list --state open --json number,updatedAt,headRefName \
       | jq -r '.[] | select((now - (.updatedAt | fromdate)) > 7200) | "\(.number)\t\(.headRefName)"'
     ```
     For each stale PR (>2h since last push) where the BUILD_STATUS row still shows IN_REVIEW:
     1. Confirm the conductor (this tab or a prior tick's tab) has a fix-request comment on the PR via `gh pr view <num> --json comments` — if there's no fix-request, leave alone (the PR may legitimately be parked).
     2. Demote the BUILD_STATUS row from IN_REVIEW to TODO. Append a salvage note to the row containing: PR number, branch name, the worktree path (`/tmp/<brief-id>`) if it still exists, and a one-line excerpt of the conductor's fix-request comment so the next worker has the context.
     3. Do NOT close the PR. A fresh worker that re-claims the brief decides whether to rebase + push the existing branch or open a new PR; closing prematurely loses their starting point.
     4. Commit `"status: demote stale IN_REVIEW <brief-id> (no push in 2h+)"` and push.
     5. **Boundary**: do not auto-demote PRs >24h old where multiple fix-request comments exist — those need a `claude-rebase-pr-<n>` recovery brief instead (#911/#905/#894/#924 precedent). Note them in the tick summary for the operator/next conductor.

6. Do not write feature code from the conductor tab. If there is process/doc work already
   in progress for the conductor role, continue it; otherwise skip to BLOCKED triage.

7. If a brief has been BLOCKED, investigate the reason and decide: expand its may-touch list (small fix in BUILD_STATUS.md + brief), or split into sub-briefs, or escalate to the human operator.
   Also sweep active manager packages under `<PLAN_DIR>/work-packages/`: parent row still CLAIMED, package heartbeat fresh, `max_depth: 1`, child count + spawn_requests within spawn budget, and child allowed files inside the parent brief's may-touch list. If a package heartbeat is stale (>30m warn, >60m consider reclaim, >90m reclaim unless human overrides), comment or update the parent row instead of adding child rows to BUILD_STATUS.md.

8. Queue replenishment:
   a. codex_todo = grep -c "| codex |.*| TODO |" <PLAN_DIR>/BUILD_STATUS.md
      claude_todo = grep -c "| claude |.*| TODO |" <PLAN_DIR>/BUILD_STATUS.md
   b. If codex_todo <= 25 (target depth 30-40; threshold per BUILD_STATUS.md operator
      amendment 2026-05-10 — verify the live header): brainstorm 3-5 NEW codex-sized briefs.
      Idea sources, in priority order: (1) `<PLAN_DIR>/MISSION_BACKLOG.md` items tagged
      `[NEW]` — name authored briefs `mission-<slug>` so the source is traceable; in the
      same commit, flip the picked backlog items' status tag from `[NEW]` to
      `[BRIEF: mission-<slug>]`. (2) BUILD_PLAN_V11.md phased plan (the v1.x / v2.x stage
      that's currently active). (3) recent PR-body "deferred to follow-up" sections.
      (4) `<PLAN_DIR>/MISSION_BACKLOG.md` gaps not yet tagged. (5) git log -100. Each brief
      follows PROTOCOL_BRIEF_TEMPLATE.md. Sizing: 200-1200 LOC, narrow may-touch,
      single-crate focus.
   c. If claude_todo <= 25 (same threshold and authoritative-source caveat as 8b): same
      for claude. Sizing: 300-2000 LOC, multi-crate OK, architectural decisions OK,
      integration tests welcome. Same `mission-<slug>` naming + backlog-tag-flip discipline
      applies.
   d. **Brief authoring discipline (tick-58 lesson)**: before publishing a brief, ground
      the may-touch list in the real codebase. Run `find crates -name '*.rs' | grep <thing>`
      or `rg -l <symbol>` to verify every file path exists. Imagined paths cause BLOCKED
      claims — 30 seconds of verification beats six dead briefs.
      Also stamp every NEW brief with the freshness header from
      `PROTOCOL_BRIEF_TEMPLATE.md` (Stream `claude-brief-freshness-header`):
      `**Authored against**: origin/main <short-sha> on <YYYY-MM-DD>`. Capture
      via `git rev-parse --short origin/main` at author-time. Forward-only —
      do NOT retrofit older briefs.
   d2. **Block-history guard (tick-102 lesson — `claude-mcp-connector-empty-dir-cleanup`)**:
      before authoring OR re-authoring a brief stub AND before inserting a TODO row,
      run `bash scripts/check-brief-block-history.sh <brief-id>`. Exit codes:
      `0` safe to author, `1` prior BLOCK on this id in git history (script prints
      SHAs + dates), `2` ledger already carries a non-TODO row for this id. If exit
      != 0, choose a different brief — do NOT re-author. The 2026-05-23 tick-92 +30
      batch re-authored a brief that had been BLOCKED earlier the same day at
      commit `7418b5045`; two claude workers then wasted iterations rediscovering
      the same impossible premise. This guard exists to make that mistake mechanical.
   e. Push ONE commit per replenishment: "briefs: replenish <owner> queue (<id1>, <id2>, ...)".
      If sourced from MISSION_BACKLOG, include the backlog-tag-flip in the SAME commit
      (so MISSION_BACKLOG and the new brief files land atomically — never re-author the
      same item across ticks).
   f. Skip if both above 25.

9. If everything is in flight and there's nothing to do, report back briefly with
   "swarm healthy, X PRs in review, Y briefs claimed, Z merged, codex_todo=A, claude_todo=B"
   and end the tick.

Do not stop the loop. Cancel only when the user asks to stop OR when the v1.0 ship gate is met: `docs/skycot/demos/v1-mission-mvp.mp4` exists AND `./scripts/smoke-skycot.sh` passes including BrowserExecutor + Slack-approval scenarios (per BUILD_STATUS.md header).
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
  rows per owner, fire if either is ≤25 (target depth 30-40,
  per BUILD_STATUS.md operator amendment 2026-05-10 — verify the
  live header at run time), write 3–5 new briefs in one commit.
  See `PROTOCOL.md` "Queue replenishment" for the idea sources,
  sizing rules, and stop condition.
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
