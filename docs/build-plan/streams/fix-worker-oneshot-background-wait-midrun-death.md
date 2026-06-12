# fix-worker-oneshot-background-wait-midrun-death

**Owner**: claude or codex — **P1 RELIABILITY (swarm-critical — 3 zombie claims in ONE day).**
**Depends on**: none.
**Authored against**: origin/main `cee947b73` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-worker-oneshot-background-wait-midrun-death

## Problem (root-caused 2026-06-12 — three independent occurrences)

Worker sessions are spawned by `scripts/claude-worker-loop.sh` as **one-shot
`claude -p` runs**. When the model ends its turn "waiting for a
notification" — a backgrounded `cargo test`/clippy task, or a
`ScheduleWakeup` — the session simply TERMINATES (`result: success`); there
is no re-invocation in `-p` mode. The brief's row stays CLAIMED forever, the
work in the `/tmp/<brief>` worktree is deleted by the loop's cleanup, and the
queue silently loses a slot-iteration.

Observed 3× on 2026-06-12 alone:

1. `launch-wire-spendgate-llm-cost-ceiling` — worker built the FULL change,
   committed `65c327324`, then ended its turn waiting on workspace clippy
   ("the waiter will notify me") + a `ScheduleWakeup(240s)` that can never
   fire. Conductor hand-salvaged the branch hours later (#2865).
2. `launch-wire-tier-quota-and-usage-metering` — died 10 min in ("Still
   compiling. I'll wait for the completion notification."), no commits,
   worktree cleaned. Row sat CLAIMED while the slot moved on.
3. The morning wave of 5 zombie CLAIMED rows (tier-quota, exit78,
   backup-cron, per-teammate-sync, wire-overage) released by the conductor at
   `031683ce8` shows the same signature: claim commit only, no work.

## Relationship to `ops-worker-supervisor-detect-zombie-wait-loop-pattern` (MERGED 2026-05-24)

NOT a duplicate (lint 2-token-overlap confirmed distinct): that detector
catches a worker SPINNING — 3+ consecutive "Waiting for..." results AND >2h
etime across iterations — and kills it. Today's failure is a SINGLE
iteration ending on its FIRST wait-result: the session dies instantly, the
loop moves on, nothing spins, so that detector never fires. DO reuse its
demote-to-TODO release machinery (see commit `b886bd9e2` + the
`claim-with-retry` release path) for layer 2 below rather than
parallel-authoring a new demote.

## Required change (two layers, both load-bearing)

1. **Prompt-side hard rule.** In the worker prompt assembled by
   `scripts/claude-worker-loop.sh` (the heredoc worker-prompt strings), add an
   explicit, prominent rule: *"You are a ONE-SHOT session. Ending your turn
   ends the session — there are NO notifications, NO wakeups. NEVER end your
   turn to 'wait' for a background task: run gates/tests in the FOREGROUND
   (blocking), or poll with a blocking `until`-loop in a single Bash call.
   `ScheduleWakeup` does nothing for you."* Place it near the
   verification/gate instructions so it is read at the moment it matters.

2. **Loop-side detector + self-heal (the backstop that makes this
   un-forgettable).** After the `claude -p` session exits, the loop already
   knows `BRIEF_ID`. Add a post-session check: if the ledger row for
   `BRIEF_ID` (on the freshly synced main) is still `CLAIMED` and no PR
   exists for the brief's branch (`gh pr list --head claude-<brief> --json
   number` empty, or the cheaper `git ls-remote origin claude-<brief>` +
   branch-tip-is-claim-commit test):
   - if the branch tip has REAL work commits beyond the claim commit: push
     the branch (preserve the work — the SpendGate case) and append a LOUD
     `[worker-midrun-death]` line to `docs/skycot/agents/SHARED.md` naming
     the brief + branch + tip SHA so the conductor salvages next tick;
   - otherwise: flip the row back `CLAIMED → TODO` (commit + push with the
     standard claim-edit machinery / `claim-with-retry.sh`-style retry) with
     a `♻️ AUTO-RELEASED (midrun death)` note, and append the same LOUD
     SHARED.md line. The queue self-heals instead of starving a brief.

3. **Tests.** Bash assertions in the style of `claim-with-retry-tests.sh`:
   (a) a fixture row CLAIMED + branch with claim-commit-only → auto-released
   to TODO + SHARED.md line written; (b) branch with a work commit → branch
   pushed (stub `git push`/`gh` via PATH or env like the #2863 harness) +
   SHARED.md line, row left CLAIMED; (c) a row that reached IN_REVIEW →
   untouched (no false positive on the happy path).

## Verify

- New assertion script green; `shellcheck` clean on touched scripts (no new
  findings vs main).
- Manual: run one supervised worker iteration end-to-end and confirm the
  happy path (ship → IN_REVIEW) takes the untouched branch.

## May touch

- `scripts/claude-worker-loop.sh` (worker prompt strings ~lines 330-390 + post-session check)
- new test/assertion script
- (read-only reference) `scripts/claim-with-retry.sh`, `scripts/stop-gate-enforce.sh`
