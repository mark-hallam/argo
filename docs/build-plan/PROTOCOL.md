# PROTOCOL — multi-agent swarm coordination

A reusable architecture for driving a multi-PR build with several AI
coding agents working concurrently on the same git repo. Bootstrapped
on the SKYCOT v1 build (22 brief slices, 21 merged PRs, ~60 minutes
of wall time). This document is self-contained: someone reading it on
a fresh project should be able to bootstrap with no prior context.

The companion artifacts:

- `PROTOCOL_LOOP_PROMPT.md` — the conductor-tick prompt to paste
  into `/loop 5m`.
- `PROTOCOL_BUILD_STATUS_TEMPLATE.md` — the seed ledger.
- `PROTOCOL_BRIEF_TEMPLATE.md` — the seed brief format.

---

## Mental model

```
                    HUMAN OPERATOR
                    (escalation only)
                          │
                  ┌───────┴────────┐
                  │   CONDUCTOR    │  one Claude tab, 5-min cron
                  │  (no codegen)  │
                  └───────┬────────┘
                          │  reads + merges
                  ┌───────┴────────┐
                  │ BUILD_STATUS.md│  the live ledger on origin/main
                  │ (lock-free)    │  every agent reads + commits
                  └───────┬────────┘
                          │  claim rows
        ┌─────────┬───────┼────────┬─────────┐
        ▼         ▼       ▼        ▼         ▼
   ┌────────┐┌────────┐┌────────┐┌────────┐┌────────┐
   │worker 1││worker 2││worker 3││worker 4││worker 5│   N tabs of
   │(claude)││(codex) ││(codex) ││(codex) ││(claude)│   Claude/Codex
   └────────┘└────────┘└────────┘└────────┘└────────┘   on worktrees
        │         │       │        │         │
        └─────────┴───────┴────────┴─────────┘
                          │  one PR per row
                          ▼
                       origin/main
```

Three roles:

1. **Conductor** — ONE Claude tab. Driven by `/loop 5m <prompt>`.
   Never writes feature code. Pulls main, reviews open PRs, merges
   clean ones, updates the ledger, unblocks BLOCKED rows, supervises
   the swarm. The conductor is the only agent allowed to merge.
2. **Workers** — N tabs (mix Claude + Codex). Each tab claims one
   row from the ledger, implements it on a worktree, opens a PR,
   marks the row IN_REVIEW, then **stops**. Do not loop — the
   conductor does the merging and the human spawns a fresh worker
   tab when capacity opens up. (A self-pacing worker is fine too,
   but expect more conflict noise.)
3. **Human operator** — escalation target only. Reviews the rare
   row the conductor flags, resolves cross-stream architectural
   choices, kills runaway tabs.

**Why two agent families?** Claude and Codex make different
mistakes; you get cheap diversity by running both. They share the
same ledger but use distinct branch prefixes (`claude-*`, `codex-*`)
so PR provenance is visible at a glance. Add a third family
(Gemini, Cursor, etc.) later by reserving its own branch prefix.

**The shared state is one file**: `BUILD_STATUS.md`, in the repo, on
`origin/main`. Every state transition is a one-line commit to that
file. No database, no service — `git push` IS the lock. First push
wins; losers pull and pick a different row.

---

## The four state machines

### 1. Brief lifecycle

```
   TODO ──claim──► CLAIMED ──open PR──► IN_REVIEW ──merge──► MERGED
                      │                     │
                      └───hit a wall────────┘
                              │
                              ▼
                           BLOCKED ──conductor unblocks──► TODO
```

A row in BUILD_STATUS.md is in exactly one of these states:

- **TODO** — available to claim. Predecessors all MERGED.
- **CLAIMED** — a worker has the row. PR not yet open.
- **IN_REVIEW** — PR open. Conductor will review next tick.
- **MERGED** — landed on main. Successors may unblock.
- **BLOCKED** — worker hit a wall it cannot resolve unilaterally.
  Reason appended to the row. Conductor decides what to do next:
  expand the may-touch list, split the brief, or escalate to human.

The legal transitions are enforced by convention, not code. Workers
pre-flight on `git pull --ff-only` before each transition; if push
loses the race, pull and pick a different row.

### 2. Conductor tick (9 steps, 5-minute cron)

Every tick the conductor runs the same numbered loop. See
`PROTOCOL_LOOP_PROMPT.md` for the exact prompt.

```
1. cd <repo> && git checkout main && git pull --ff-only
2. git log --oneline origin/main -8     (what landed since last tick)
3. gh pr list --state open               (what's in flight)
4. cat BUILD_STATUS.md | head -50        (queue state)
5. For each open PR from a worker branch you haven't reviewed:
   - gh pr diff <num> | head -300
   - Verify against the brief's acceptance checklist
   - Clean → squash-merge, delete branch, update ledger row
   - Issues → gh pr comment with the specific problem
6. Continue any conductor-owned in-progress work (rare)
7. For each BLOCKED row: investigate, expand may-touch, split, or
   escalate
8. Queue replenishment: count owner.TODO; if either ≤3 brainstorm
   3-5 new briefs and push as one commit per owner. See
   "Queue replenishment" below for details.
9. If nothing to do, report briefly + end the tick
```

Two rules that matter:

- **Never write feature code from the conductor tab.** It edits only
  BUILD_STATUS.md (status updates) and merges PRs. Mixing the
  conductor with a worker has bitten the swarm twice — see the
  webinbox anti-pattern below.
- **Always read from a fresh `git pull` of origin/main, not a stale
  local copy.** Workers are pushing to main constantly. The
  conductor tab's local working tree is allowed to be slightly
  stale; it just must `git pull --ff-only` at the top of every
  tick before reading or committing anything.

### 3. Worker tick

```
1. git checkout main && git pull --ff-only
2. cat BUILD_STATUS.md
3. Find the first row where:
     owner matches my agent family
     status == TODO
     all predecessors MERGED
4. Edit BUILD_STATUS.md → set that row's status to CLAIMED
5. git commit -m "status: claim <brief-id>" && git push origin main
   (if push fails: pull, pick a different row, retry)
6. Read streams/<brief-id>.md
7. git worktree add /tmp/<brief-id> origin/main
8. cd /tmp/<brief-id> && git checkout -b <owner>-<brief-id>
9. Implement the brief. Stay inside the may-touch list.
10. cargo build / clippy / test (or project equivalent). Tick the
    acceptance boxes. Verify file LOC budget.
11. git push -u origin <owner>-<brief-id>
12. gh pr create with the brief's PR title, body listing checked
    acceptance boxes
13. Edit BUILD_STATUS.md (in the main checkout, not the worktree)
    → status = IN_REVIEW, add PR number
14. git commit -m "status: <brief-id> in review (#NNN)" && git push
15. STOP. The conductor merges. A fresh tab starts the next brief.
```

The single most important worker rule: **one PR per tab, then stop**.
Don't loop. Don't claim a second row from the same tab. The
conductor has visibility on what's mergeable; the worker doesn't.

### 4. The unblock dance

When a worker hits a wall (clippy fails, the brief's may-touch list
is too tight, or it discovers it needs to edit a workspace
`Cargo.toml` that the brief forbade):

```
worker: edit BUILD_STATUS.md row to BLOCKED with reason
worker: git commit "status: <brief-id> blocked" && push
worker: pick a different brief, or stop
                  │
        next tick │
                  ▼
conductor: see BLOCKED row in step 7 of the tick
conductor: read the reason, decide:
            (a) expand brief's may-touch list (most common — Cargo.lock,
                workspace Cargo.toml deps, sibling crate one-line edit)
            (b) split brief into smaller sub-briefs (H-style)
            (c) flag for human (mission disagreement, contract change)
conductor: edit brief + BUILD_STATUS.md, commit
                  │
        next tick │
                  ▼
worker tab (fresh): see row back at TODO with expanded scope, claim
```

A typical block-unblock cycle is one or two ticks. The conductor's
unblock commits during this run included:

- `status: unblock C-mcp-wire (Cargo.toml dep add now allowed)`
- `status: unblock C-mcp-wire (Cargo.lock now in may-touch list)`
- `status: unblock all 4 D-* briefs (lib.rs one-line module add allowed)`
- `status: D-push unblocked (use branch codex-D-push-v2)`
- `status: unblock C-gmail (Cargo.lock + defer keyring); apply same
   Cargo.lock allowance to all 5 C-* connector briefs`

These are the seams the brief author didn't anticipate. Expect one
per stream.

---

## Why it works

**First-commit-wins on BUILD_STATUS.md is sufficient.** Two workers
that both pull main, both read TODO row X, both edit the row to
CLAIMED, both `git push`: the second one's push fails fast-forward.
That worker pulls, sees the row is now CLAIMED by the other, and
picks a different row. No mutex, no service, no state machine
beyond `git push`. We hit this race ~5 times in 60 minutes of swarm.
Cost: one extra pull per loser. Free correctness.

**The file partition + per-brief may-touch list prevents most
conflicts.** Each brief lists "files you may touch" and "files you
must NOT touch". Different streams get different files. The hot
path of conflicts is a small set of shared seams (Cargo.toml /
Cargo.lock / lib.rs / mod.rs / BUILD_STATUS.md), and those are
either:

- explicitly opened in the brief (one-line module add, narrow dep
  add), or
- merged by the conductor manually when two PRs both touch them
  (3-way merge in a worktree, ~30 seconds).

The brief author over-restricts on purpose; the conductor unblocks
when a worker bumps into something the brief forgot. Better than
a silently-too-permissive brief that lets workers stomp each other.

**Sub-slicing kept the integration capstone tractable.** Stream H
(auto-approval + orchestrator + skill library) was three coupled
deliverables. Splitting into H-1 / H-2 / H-3 / H-4a / H-4b / H-4c
let one worker land each independently — five PRs in series in one
tab while the conductor merged each before the next started. The
ledger row tracked the slice list:
`H-1 ✓ #796 + H-2 ✓ #800 + H-3 ✓ #799 + H-4a ✓ #801 + H-4b ✓ #803`.
A monolithic H brief would have stalled.

**No central registry of agent identity.** `git config user.email`
and the `Co-Authored-By` trailer carry the provenance. Branch
prefixes (`claude-*`, `codex-*`) carry the family. The owner column
in BUILD_STATUS.md says who is supposed to pick the row. That's it.

---

## Queue replenishment — keeping the swarm fed indefinitely

The original SKYCOT plan had a finite queue: 22 briefs across the
14-day v1. The conductor tick as originally written merged PRs and
unblocked rows but never wrote new ones. Once the last brief was
MERGED, every worker tab would idle and the swarm would die.

**Queue replenishment fixes that.** Step 8 of the conductor tick
runs every cycle and asks one question per owner: is the TODO queue
shallow? If yes, the conductor brainstorms a fresh batch of briefs
from the project's mission and pushes them in one commit. The
swarm self-extends.

The trigger fired naturally for the first time when `claude_todo`
dropped to 0 — the conductor wrote 4 new claude briefs
(claude-cu-skill-learn-wire, claude-tui-h-stack-panels,
claude-e2e-integration-test, claude-genesis-llm-refinement) and
pushed them as a single commit. The queue refilled in one tick.

### The trigger

```
[every conductor tick]
        │
        ▼
count(owner.TODO)
        │
        ├── ≤ 3  → brainstorm + write 3-5 briefs → push one commit
        └── > 3  → skip
```

Concretely, at step 8 the conductor runs:

```
codex_todo  = grep -c "| codex |.*| TODO |"  <PLAN_DIR>/BUILD_STATUS.md
claude_todo = grep -c "| claude |.*| TODO |" <PLAN_DIR>/BUILD_STATUS.md
```

and fires the replenishment branch for each owner whose count is
≤3. Both queues above 3 → the step is a no-op.

### Idea sources

When the trigger fires, the conductor brainstorms from four sources:

1. **`docs/skycot/MISSION.md`** (or whatever your project's mission
   doc is) — the long arc of the product. Pick a capability that is
   not yet in main.
2. **Recent merged-PR bodies' "deferred to follow-up" sections.**
   Workers routinely flag work they couldn't fit in their brief; the
   conductor mines those for ready-to-go briefs.
3. **The gap between the v1 demo and current main.** What does the
   demo claim that the code doesn't yet do? Each gap = one brief.
4. **`git log --oneline origin/main -100`** — recent commits often
   imply natural follow-ups (a new module that needs a test crate,
   a CLI that needs a `--json` flag, a metric that needs a reset).

### Sizing — codex vs claude

The two agent families have different strengths. Briefs target the
strength of their owner.

| Owner   | LOC budget | Scope              | Acceptance flavor                |
|---------|------------|--------------------|----------------------------------|
| codex   | 200–1200   | Single-crate, narrow may-touch list | Mechanical, well-defined; mirror existing C/D/H briefs |
| claude  | 300–2000   | May span crates; architectural decisions OK | Integration tests welcome; mirror existing claude-* briefs |

Both follow `docs/build-plan/PROTOCOL_BRIEF_TEMPLATE.md`. Both go
into `streams/<brief-id>.md` and get a row added to
`BUILD_STATUS.md`.

### The push convention

One commit per owner per replenishment tick. Subject:

```
briefs: replenish <owner> queue (<id1>, <id2>, <id3>, ...)
```

Example: `briefs: replenish claude queue (claude-cu-skill-learn-wire,
claude-tui-h-stack-panels, claude-e2e-integration-test,
claude-genesis-llm-refinement)`.

The commit edits `BUILD_STATUS.md` (adding new TODO rows) and adds
the brief files in `streams/`. Workers see the new rows on their
next pull.

### Stop condition

Replenishment is unbounded by default. The conductor cancels the
loop only when **every brief in BUILD_STATUS.md is MERGED AND the
v1 demo gate is green** — for SKYCOT that means
`claude-e2e-integration-test` passing on main and the demo script
runnable end-to-end. This represents v1 customer-launch readiness;
the conductor knows it's safe to stop because the product can ship.

If the human operator wants to stop earlier, ask the conductor in
the chat — the loop responds to a kill instruction at the top of
the next tick.

---

## Anti-patterns we hit and how we recovered

These are real bugs from the SKYCOT run. Expect them on any swarm.

### Two parallel agents claimed the same row

`status: claim C-mcp-wire` appears 5 times in the log (`a87db42`,
`088436b`, `9f4c691`, `55a583c`, …). Each time, the second worker
saw the row was now CLAIMED on its next pull, abandoned, and picked
a different row. **Recovery: free, no human action needed.** First
commit wins.

### Two parallel agents implemented the same H sub-slice

Both #796 and a duplicate were opened against H-1. Conductor closed
the duplicate on review (kept the more comprehensive PR).
**Recovery: conductor's discretion in step 5 of the tick. 30
seconds.** Lesson: when conductor approves the claim of a multi-
slice brief, it should immediately update the ledger row to list
the sub-slices it expects, so workers don't double-claim within the
brief.

### `Cargo.toml` `workspace.members` conflicts every time a new crate landed

Every Stream D channel + Stream C connector added a new crate. Each
PR edited workspace `members = [...]`. PRs merged in arbitrary
order; second-to-merge always conflicted on that line. **Recovery:
conductor opens a worktree on origin/main, runs `git rebase
origin/main` on the conflicting branch, hand-resolves the workspace
list, force-with-leases the branch, and re-merges.** ~30 seconds
per conflict. Five conflicts total over the run.

Fix for next time: lock workspace.members in a brief, OR have the
conductor pre-add empty crate dirs at the start of the run so
worker PRs only add files to existing crates.

### A conductor-tick commit accidentally swept up a worker's WIP from the working tree

The webinbox bug. Worker had unstaged webinbox files in the
conductor's working tree. The conductor ran `git commit -am "status:
…"` with `-a` (which stages all tracked-file modifications). The
status-update commit shipped 7 untracked-file additions. The worker
came back, saw its files already on main, and the open PR became
redundant. **Recovery: the row was marked MERGED with a note
explaining what happened; the open PR was closed as redundant
(content was identical to main).**

Fix: **always run the conductor tab from a separate worktree on
origin/main, never the main checkout.** Or use `git add <file>` +
`git commit -m` (not `-am`) for status updates. The
`PROTOCOL_LOOP_PROMPT.md` explicitly stages only BUILD_STATUS.md.

### Branch contention — codex-D-push was stale

A codex worker had opened PR #785 on branch `codex-D-push`. The
branch went stale (rebased badly, conflicts piled up). Conductor's
unblock: allow the worker to use `codex-D-push-v2` and close #785.
**Recovery: one-line ledger update naming the new branch. Worker
re-implemented from fresh main.**

Lesson: when a PR has been open >2 hours with no progress, the
conductor should ping it; if no response in another tick, close
the PR and let a fresh worker re-claim.

### Brief shape didn't match reality (executor field schema)

Stream A locked the executor-field schema in the manifest. Stream
C-mcp-wire later needed a related but different schema for
`[mcp_servers]`. The original brief said "do NOT modify any
existing declaration" in argo-vertical. C-mcp-wire blocked twice
asking for permission to add a new section. **Recovery: conductor
expanded the brief in two iterations** (`status: unblock C-mcp-wire
(allow narrow argo-vertical [mcp_servers] schema add)`).

Lesson: contract briefs (Stream A here) define the trait/schema
shape; downstream briefs that touch that crate get a "narrow add"
clause for new sections. Bake into the template.

---

## The roles, in detail

### Conductor

- ONE Claude tab. Driven by `/loop 5m <PROTOCOL_LOOP_PROMPT.md>`.
- Allowed to: pull main, read BUILD_STATUS.md, read PR diffs, merge
  clean PRs, comment on PRs with issues, edit BUILD_STATUS.md
  (status transitions only), edit briefs in `streams/` (to expand
  may-touch lists when unblocking), close stale PRs, **write new
  briefs when a TODO queue runs shallow** (queue replenishment —
  see section above).
- NOT allowed to: write feature code, modify other files, modify
  workspace deps, push to anything other than main.
- Tick budget: ~3 minutes per tick. The 5-minute interval gives 2
  minutes of slack; longer ticks usually mean the conductor got
  drawn into hand-fixing a merge conflict, which is fine
  occasionally but a smell if it becomes routine.
- Stop condition: every brief MERGED **and the v1 demo gate is
  green** (e.g. the integration-test brief passes), or human
  intervention. Queue replenishment keeps the loop alive
  indefinitely otherwise.

### Worker (Claude or Codex)

- One tab per brief. Stops after one PR.
- Allowed to: pull main, claim a TODO row, read its brief,
  implement on a worktree branch, push the branch, open a PR,
  mark the row IN_REVIEW. Mark BLOCKED with reason if stuck.
- NOT allowed to: merge anything, modify another brief's files,
  modify the conductor's BUILD_STATUS.md transitions out of order,
  delete or force-push another agent's branch.
- Stop after the IN_REVIEW commit. Do not loop.

### Human operator

- Spawns worker tabs. Cancels runaway tabs. Reviews any row the
  conductor escalates.
- Resolves architectural decisions the conductor isn't authorized
  to make (e.g. mission-level pivots, security trade-offs,
  third-party API choices not pinned in the brief).
- Watches the ledger periodically. If the conductor is stuck in a
  loop (same BLOCKED row two ticks running), intervenes.

---

## The artifacts

Three files in `docs/build-plan/` (or wherever your repo's plan
lives). All committed to the repo, all read/written by every agent.

### `BUILD_STATUS.md` — the ledger

Single file, one table, one row per brief. Plus three appendix
sections: dependencies graph, blocked rows, done rows. Every state
transition is a commit to this file. See
`PROTOCOL_BUILD_STATUS_TEMPLATE.md` for the exact shape.

Commit-message convention is mandatory:
- `status: claim <brief-id>`
- `status: <brief-id> in review (#<PR-number>)`
- `status: <brief-id> merged (#<PR-number>)`
- `status: <brief-id> blocked`
- `status: unblock <brief-id> (<reason>)`

This makes `git log --oneline origin/main` a readable swarm
timeline, which is what the conductor reads at step 2 of every
tick.

### Brief files in `streams/<brief-id>.md`

One brief per row in the ledger. The shape is fixed. See
`PROTOCOL_BRIEF_TEMPLATE.md`. Required sections:

- Owner / Depends on
- Scope (1–2 paragraphs)
- Files you may touch
- Files you must NOT touch
- Acceptance criteria (checkboxes)
- Branch + PR title convention
- Notes for the implementer

The "may touch" list is the brief's most important section. Tight =
fewer conflicts but more BLOCKED unblock cycles. Loose = more
conflicts but smoother flow. Start tight; the conductor expands as
unblocks happen.

### `CONTRIBUTING_AGENT.md` — worker rules

The worker-side protocol. Each worker reads this once before
starting. Covers: how to claim, the gates before opening a PR
(build / lint / test / LOC budget), branch + PR conventions, what
to do when stuck, the rules-that-override-everything list (no
force-push to main, no editing other agents' branches, etc.).

The exact text used on SKYCOT lives at
`docs/build-plan/CONTRIBUTING_AGENT.md` and is short enough (~140
lines) that workers can paste it into context every session.

---

## The git workflow

### Worktree-per-PR

Every worker creates its own worktree before touching code:

```
git worktree add /tmp/<brief-id> origin/main
cd /tmp/<brief-id>
git checkout -b <owner>-<brief-id>
# ... implement ...
```

Why: the conductor tab and other workers may all be checked out on
the same repo's `main`. Separate worktrees mean separate working
trees. No `git stash` dance, no accidental staging across briefs.

The conductor uses its own worktree too (`/tmp/conductor` or just
its tab's main checkout, kept clean — never editing files there).

### Direct-push to main for ledger updates only

Status updates are one-line edits to BUILD_STATUS.md. They do not
go through PR review:

```
git checkout main
git pull --ff-only
# edit BUILD_STATUS.md
git add docs/build-plan/BUILD_STATUS.md
git commit -m "status: claim X"
git push origin main
```

Code changes always go through a PR. Only ledger transitions
direct-push.

### Squash-merge for code PRs

`gh pr merge <num> --squash --delete-branch`. The squashed commit
title becomes the merge-commit subject; preserve the brief id in
parens (e.g. `executor: trait + manifest enum (A-executor-trait)`).
This makes `git log --oneline` itself a readable feature timeline.

### Force-with-lease for rebase pushes

When the conductor hand-resolves a workspace.members conflict on a
worker branch:

```
cd /tmp/<brief-id>
git fetch origin
git rebase origin/main
# resolve conflict in Cargo.toml
git rebase --continue
git push --force-with-lease origin <branch>
```

`--force-with-lease`, never plain `--force`. Protects against
clobbering a worker that may have pushed something the conductor
hadn't fetched yet.

### Per-file 500-LOC budget

Hard constraint enforced by every worker's acceptance checklist
and verified by the conductor on PR review. Encourages workers to
split implementations across multiple sibling files
(`worker_actions_executor.rs` + `worker_actions_executor_tests.rs`
+ `worker_actions_dispatch.rs`, etc) rather than monolithing.
Compounding effect: smaller diffs → faster review → less merge-
conflict surface area.

---

## Bootstrapping on a new project

1. Create `docs/build-plan/` (or your project's equivalent).
2. Copy `PROTOCOL.md`, `PROTOCOL_LOOP_PROMPT.md`,
   `PROTOCOL_BUILD_STATUS_TEMPLATE.md`,
   `PROTOCOL_BRIEF_TEMPLATE.md` into it.
3. Rename the templates: `PROTOCOL_BUILD_STATUS_TEMPLATE.md →
   BUILD_STATUS.md`, etc, and fill in your project's brief rows.
4. Write one brief per intended PR slice in `streams/<brief-id>.md`.
   Tighten may-touch lists; the conductor will expand later.
5. Write a tight `CONTRIBUTING_AGENT.md` adapting the worker rules
   to your toolchain (replace `cargo build/clippy/test` with your
   build commands, keep the LOC budget if you want it).
6. Open one Claude tab. Paste `PROTOCOL_LOOP_PROMPT.md` content
   into a `/loop 5m` command, substituting your repo path. Hit
   enter. The conductor is now running.
7. Open N worker tabs (mix of Claude and Codex). In each, paste:
   "Read `docs/build-plan/CONTRIBUTING_AGENT.md` and pick the
   first TODO row matching your agent family." Hit enter.
8. Watch the ledger. Spawn fresh worker tabs as ones finish.

That's it. The swarm is live.

---

## Sizing and pacing

A rough rule of thumb from the SKYCOT run:

- **Brief size**: 1 brief = ~1 PR = ~200–500 LOC = ~30–60 minutes
  of one worker tab.
- **Conductor cadence**: 5-minute tick fires ~12 times/hour. ~50%
  of ticks are no-ops ("nothing to merge, nothing to unblock").
  ~30% merge one PR. ~15% merge multiple PRs or unblock a row.
  ~5% require human escalation.
- **Worker concurrency**: 4–6 active workers is a sweet spot for
  one human operator. More than 8 and the conductor's PR review
  queue starts to back up; the merge becomes the bottleneck.
- **Throughput on a clean run**: 1 PR every 3 minutes of wall time
  is achievable when the dependency graph is fanned-out (e.g.
  Stream C connectors fanning out off C-mcp-wire). Sustained
  throughput is closer to 1 PR every 5–8 minutes once you average
  in serial dependencies.

If the swarm stalls (no merges in two consecutive ticks, no open
PRs, multiple BLOCKED rows): the conductor or human should cut a
new tighter brief or split a fat one. The swarm runs on a steady
diet of small briefs; starve it and it dies.
