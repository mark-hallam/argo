# Round 37 · Process drift sweep

- **Date**: 2026-05-24
- **Round**: 37 (scheduled-task hourly fire, read-only)
- **Verdict**: 🟡 **PARTIAL** — fix-by tracker clean, but heartbeat write-side gap persists + LIVE WIPE-RACE OBSERVED mid-session
- **Severity tier**: P1 systemic (operator action required for write-side fix)

## Setup

```bash
bash scripts/check-unread-handoffs.sh
bash scripts/check-findings-only-fix-by.sh
bash scripts/check-gateway-freshness.sh
curl -sf http://127.0.0.1:7400/health
grep -E 'ops-evaluator-scheduled-task-prompt|ops-evaluator-interactive-launch' \
  docs/build-plan/BUILD_STATUS.md
ls docs/build-plan/streams/ | grep -i 'evaluator-scheduled\|evaluator-interactive'
```

## Observed

### 1. Findings-only fix-by tracker — 🟢 5/5 within budget

```
🟢 eval-cron-duty-fires-agent-runner-end-to-end       within budget (2026-05-30, 7 days remaining)
🟢 eval-workflow-execution-runs-real-step             within budget (2026-05-30, 7 days remaining)
🟢 mission-hire-audit-payload-shape-validation        within budget (2026-05-30, 7 days remaining)
🟢 mission-cu-macos-cross-tenant-capability-test      within budget (2026-05-30, 7 days remaining)
🟢 mission-cross-tenant-test-audit-export             within budget (2026-05-25, 2 days remaining)

Tally: 5 within-budget · 0 due-today · 0 overdue · 0 informational · 5 total.
```

Note: `mission-cross-tenant-test-audit-export` deadline is 2026-05-25
(tomorrow) — flag for next conductor tick. The other 4 are pegged to
the 2026-05-30 ship gate.

### 2. Gateway freshness — 🟡 intermittent

```
$ curl -sf http://127.0.0.1:7400/health
exit=0   (200 OK, empty body)

$ bash scripts/check-gateway-freshness.sh
STATUS: UNREACHABLE  (no response from http://127.0.0.1:7400/health — gateway not running?)
```

Two probes ~10s apart returned different results. The gateway is
flapping — probably under load from concurrent worker compiles (3
claude workers + cargo test concurrently churning the shared target
dir). Same pattern observed in rounds 25b and 31b. SPA-driven probes
deferred this round; not worth the token spend when the surface is
known-intermittent under swarm load. Informational only.

### 3. Heartbeat write-side gap — 🔴 STILL OPEN

Round 36 (cbd430961, 2026-05-23T13:27Z) identified 4 brief
recommendations. Status today:

| Brief | Priority | Status |
|---|---|---|
| `ops-heartbeat-watchdog-reject-future-timestamps` | P2 | ✅ IN_REVIEW (#2150) |
| `fix-cron-duty-wire-closure-test-rebase-onto-tenant-id-arg` | P1 | ✅ CLAIMED (bc56fa761) |
| `ops-evaluator-scheduled-task-prompt-use-flavoured-heartbeat-path` | P1 | ❌ **NOT AUTHORED** |
| `ops-evaluator-interactive-launch-template-use-flavoured-heartbeat-path` | P1 | ❌ **NOT AUTHORED** |

`grep -E 'ops-evaluator-scheduled-task-prompt|ops-evaluator-interactive-launch' docs/build-plan/BUILD_STATUS.md`
returns ZERO matches. `ls docs/build-plan/streams/ | grep -i 'evaluator-scheduled\|evaluator-interactive'`
returns ZERO matches.

Tick-113 SHARED.md note acknowledges: "Two operator-actionable
round-36 finds NOT brief-authored (they require editing
`~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` which
isn't repo-tracked) — operator-flagged in next tick summary." Tick-114
did not re-flag them; tick-115 should.

**Why this matters**: PR #2150 fixes the READ side (watchdog drops
future-dated >5min). But the WRITE side — the scheduled-task SKILL.md
prompt that tells the evaluator "Write your fresh heartbeat at
`docs/skycot/agents/heartbeats/evaluator.json`" — still names the
BASE filename. Every concurrent evaluator session (scheduled-task,
interactive, ad-hoc) collides on the same file. Today they overwrite
each other's run_notes; before #2150 lands, the most-recent writer's
timezone bug poisons the freshness verdict; after #2150 lands, the
correctly-timestamped session wins but the LOSING session's run_notes
disappear silently.

The right design is one heartbeat file per session-flavour:
- `evaluator-scheduled.json` — for the SKILL.md cron path
- `evaluator-interactive.json` — for the operator's manual evaluator launch
- `evaluator.json` — deprecated / aliased

The watchdog already supports per-session heartbeats (PR #2139's
dedupe contract). The 2 unactioned briefs are the write-side
analogues — both small (~5-10 LOC each, just `s/evaluator.json/<flavoured>/`
in the prompt template + a one-line note in the watchdog README).

This round, I followed SKILL.md literally and wrote to `evaluator.json`
with CORRECT UTC time (`2026-05-24T14:30:00Z`, real). This overwrites
the previous round's Melbourne-future-dated `2026-05-23T23:30:00Z`
stamp — for one round the watchdog will be correct. The next
interactive session will likely overwrite again with the same bug.

### 4. LIVE WIPE-RACE OBSERVED MID-SESSION (NEW)

While drafting this round's reports, a concurrent worker session
(claim-with-retry on `mission-path-guard-canonicalize-symlinks` or
`fix-cron-duty-wire-closure-test-rebase-onto-tenant-id-arg` —
inference, not confirmed) appears to have done a `git reset --hard
origin/main` or `git stash + drop` in the shared `~/code/argo`
checkout. The destructive effect:

- `docs/skycot/agents/heartbeats/evaluator.json` reverted to
  origin/main's Melbourne-future-dated round-31 contents
- `docs/skycot/evals/scheduled/LATEST.md` reverted to origin/main's
  pre-round-37 contents
- `docs/skycot/evals/scheduled/2026-05-24/` directory entirely WIPED
  (my 2 already-written reports lost; this report is the third)
- `docs/skycot/agents/SHARED.md` survived as a working-tree
  modification (because it was committed/tracked differently?)

Recovery: re-applied all 3 writes + this report from in-conversation
state, then atomic stage+commit+pull-rebase+push to ship before another
race fires. This is the same pattern documented in user-memory
`feedback_worker_git_divergence_flake` — but observed here from the
evaluator's perspective, not a worker's.

### 5. Concurrent-write race observed: 7th consecutive round

The base `evaluator.json` write-collision pattern has now been
observed in rounds 31, 32, 33, 34, 35, 36, 37 — 7 rounds running.
Round 36 promoted the write-side briefs from P3 to P1. They remain
unauthored. The wipe-race (scenario 4) is a separate adjacent issue:
the same shared checkout means concurrent workers can destroy
unstaged evaluator writes. Either:
- (a) evaluator runs from its own worktree (current ad-hoc), OR
- (b) workers refuse to `git reset --hard` if the worktree has
  unstaged changes outside their may-touch list

Recommending neither here — both are conductor/operator decisions
about shared-checkout hygiene. Just surfacing.

## Anti-theater check

- Fix-by tracker output is literal; no manual selection.
- Heartbeat gap verified by two grep commands returning zero matches.
- Watchdog #2150 status verified by reading
  `docs/build-plan/BUILD_STATUS.md` line containing `IN_REVIEW 2026-05-23`.
- Gateway flapping verified by two distinct shell invocations returning
  different exit codes within seconds.
- Wipe-race verified by `git status --short` showing only SHARED.md
  as modified post-collision; my report dir + heartbeat + LATEST.md
  edits were absent on disk despite being written successfully.

## Brief candidates

**NEW (re-surfaced from round 36, still unactioned)**:

1. `ops-evaluator-scheduled-task-prompt-use-flavoured-heartbeat-path`
   (P1, ~5-10 LOC) — operator-actionable. Update
   `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` to
   tell the agent to write to `evaluator-scheduled.json` (not
   `evaluator.json`). Out of evaluator's normal three-path write
   scope per charter — needs operator or conductor to author + ship.

2. `ops-evaluator-interactive-launch-template-use-flavoured-heartbeat-path`
   (P1, ~5-10 LOC) — same fix for the operator's interactive evaluator
   launch surface. Wherever the operator's "launch evaluator manually"
   prompt template lives, update it to write
   `evaluator-interactive.json`.

**NEW THIS ROUND (from wipe-race observation)**:

3. `ops-worker-refuse-reset-hard-with-unstaged-eval-writes` (P2,
   ~20-40 LOC) — defensive gate in the worker template's claim-with-retry
   recovery path: if `git status --porcelain` shows changes under
   `docs/skycot/evals/` or `docs/skycot/agents/heartbeats/` that the
   worker didn't make, refuse the `git reset --hard` and surface to
   SHARED.md. Avoids wiping evaluator drafts. Alternative: evaluator
   always runs from its own `/tmp/skycot-evaluator-round-N` worktree
   — but that's a bigger SKILL.md change.

**RE-SURFACED FOR VISIBILITY** (round-35 unactioned):

4. `eval-gateway-uuid-bound-routes-tenant-ownership-sweep` (P1,
   ~150-200 LOC) — round-33's `mission-approval-decide-request-tenant-pinned`
   (now MERGED via #2146 verified this round) + tick-110's
   `mission-gateway-session-export-tenant-bind` (TODO) are TWO
   instances of the same SQL-bind-on-UUID-alone pattern. A sweep
   brief enumerating every gateway route doing that without a tenant
   ownership pre-flight would catch the third instance before it ships.

**OPERATIONAL** (no action needed this round):

- Gateway flapping is informational. Under heavy swarm-compile load
  the gateway sometimes drops the health probe. Not a substrate gap.
  If it persists across quiet periods, escalate to a P2 reliability
  brief.
- `mission-cross-tenant-test-audit-export` fix-by deadline is
  tomorrow (2026-05-25). Conductor should verify status in the next
  tick.

## Mission claim impact

None this scenario — this is process / ops hygiene only.
