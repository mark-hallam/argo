# PROTOCOL_BUILD_STATUS_TEMPLATE — seed BUILD_STATUS.md

Copy this file to `BUILD_STATUS.md` (in the same directory) on a
new project, fill in the queue rows for your briefs, commit it to
`main`. Every agent reads + writes this file from then on.

See `PROTOCOL.md` for the architecture, `PROTOCOL_LOOP_PROMPT.md`
for the conductor's tick, and `PROTOCOL_BRIEF_TEMPLATE.md` for the
brief shape.

---

```markdown
# Build status — live ledger

**Updated by every agent on every state transition.** Always read
from `origin/main`; never trust a stale local copy.

Status values: `TODO` (claim me) · `CLAIMED` (someone is working) ·
`IN_REVIEW` (PR open) · `MERGED` (on main) · `BLOCKED` (needs
unblock).

When you claim or update a row, commit `BUILD_STATUS.md` with
message `status: <action> <brief-id>` (e.g. `status: claim
<brief-id>`, `status: <brief-id> in review (#<PR>)`,
`status: <brief-id> merged (#<PR>)`, `status: <brief-id> blocked`).

Stage **only** `BUILD_STATUS.md` (and, when unblocking, the brief
file) — never `git commit -am`. The conductor enforces this; a
sweep of the working tree has bitten the swarm before.

---

## Active queue

| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| <brief-id-1> | claude | TODO | — | (not yet started) |
| <brief-id-2> | codex | TODO | — | (waits on <brief-id-1>) |
| <brief-id-3> | codex | TODO | — | (waits on <brief-id-1>) |

Owner values: `claude`, `codex`, or your additional agent family
prefix. Conductor uses no owner column — it merges everyone's PRs.

---

## Picking your next brief

Workers pick the first row where:
- `owner == <my agent family>`
- `status == TODO`
- All "depends on" predecessors have `status == MERGED`

If multiple agents try to claim the same row, the one whose commit
lands first on main wins. The losers will see the row is already
CLAIMED on their next pull and pick a different one.

If no row matches, stop the tab. The conductor or human will spawn
fresh work.

---

## Known dependencies

Encode the dependency graph as ASCII art so workers can verify
predecessors at a glance:

```
<brief-id-1>
    ├── <brief-id-2>
    └── <brief-id-3>

<brief-id-4>   (no dependencies)
    └── <brief-id-5>
```

The "depends on" column above is the source of truth; this graph
is illustrative.

---

## Blocked rows

(none currently)

When marking BLOCKED, append a short reason on the same row:
`status=BLOCKED reason="cargo clippy fails on field X; need
Stream Z's trait shape first"`.

Conductor's job at the next tick: read the reason, decide
(a) expand may-touch list, (b) split into sub-briefs, or
(c) escalate to human.

---

## Done rows

(none currently)

When the queue is fully MERGED, append a `Plan complete` note here
with the date and a link to the deliverable (demo recording,
release tag, postmortem doc).
```

---

## Commit-message convention (mandatory)

Every status transition commits `BUILD_STATUS.md` with one of these
exact verbs in the subject line:

- `status: claim <brief-id>` — TODO → CLAIMED
- `status: <brief-id> in review (#<PR>)` — CLAIMED → IN_REVIEW
- `status: <brief-id> merged (#<PR>)` — IN_REVIEW → MERGED (conductor)
- `status: <brief-id> blocked` — any → BLOCKED
- `status: unblock <brief-id> (<reason>)` — BLOCKED → TODO (conductor)
- `status: <brief-id> TODO (<reason>)` — re-opening, e.g. after PR closed

This makes `git log --oneline origin/main` itself a swarm timeline.
The conductor reads this log at step 2 of every tick and uses it
to figure out what landed since last tick.

---

## Optional: counts at the top

For long-running plans, prepend a one-line summary above the queue
table that the conductor updates periodically:

`**Progress:** 21 / 22 briefs MERGED · 1 CLAIMED (H-4c)`

This is purely informational. The queue table is the source of
truth.

---

## What NOT to do

- Don't put PR review notes in this file. They go in `gh pr
  comment` against the PR.
- Don't put architectural debate here. Use the brief's notes
  section, or escalate to human in the Blocked rows section.
- Don't track sub-task completion at the row level for fat briefs.
  Either the brief is small enough to be one row, or split it into
  separate rows (each its own brief file).
- Don't delete rows. Once a row exists, it ends in MERGED or as a
  documented closure note. The ledger is part of the audit trail.
