# Write-side runbook — heartbeat timestamps

Companion to `README.md` "Future-dated rejection (5-min grace)"
section (read-side defence, shipped via PR #2150). This document
covers the **operator-actionable** write side: stop the agents
from emitting wrong-time stamps in the first place.

If you only do one thing: confirm every scheduled-task `SKILL.md`
on this host stamps `date -u +"%Y-%m-%dT%H:%M:%SZ"` — never
`date +"%Y-%m-%dT%H:%M:%SZ"`. The `-u` flag is the critical bit.

## The bug

A scheduled-task prompt that calls `date +"%Y-%m-%dT%H:%M:%SZ"`
(no `-u`) interpolates the host's **local** time but stamps it
with a literal `Z` suffix that asserts UTC. On a host in Melbourne
(AEST = UTC+10), 13:27 local gets written as
`2026-05-23T23:30:00Z` — 10 hours in the actual future.

Evaluator round-36 (2026-05-23) observed this race live:

- A concurrent operator-interactive evaluator session wrote
  `last_run_utc: "2026-05-23T23:30:00Z"` when real UTC was 13:27.
- The watchdog's per-session dedupe (from PR #2139) picks the
  lexicographically-largest timestamp.
- The future-dated row won the verdict; the agent showed
  ✅ alive with `AGE = -35735s` (~-10h).

By round-37 the gap had been observed for **seven consecutive
evaluator rounds** without remediation — a process-drift signal
the operator may not have known about because the brief that
should have remediated it (`ops-evaluator-scheduled-task-prompt-
use-flavoured-heartbeat-path`) was promised but never authored.
This runbook closes that chain.

## Right vs wrong (one-liner each)

WRONG — interpolates Melbourne local, stamps `Z`:

```bash
TIMESTAMP=$(date +"%Y-%m-%dT%H:%M:%SZ")
```

RIGHT — `-u` forces UTC, stamp matches the suffix:

```bash
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
```

Single character (`-u`). Catches every tz mismatch
regardless of host timezone.

## Where to apply the fix

The scheduled-task prompts live at:

```
~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md
~/.claude/scheduled-tasks/skycot-conductor-hourly/SKILL.md
~/.claude/scheduled-tasks/skycot-auditor-3x-daily/SKILL.md
~/.claude/scheduled-tasks/skycot-marketing-weekday-morning/SKILL.md
~/.claude/scheduled-tasks/skycot-personal-site-weekday-morning/SKILL.md
```

These files are **operator-actionable** — they're not tracked in
the repo. The audit + fix is a one-time operator pass:

```bash
grep -lrn 'date +"%Y-%m-%dT%H:%M:%SZ"' ~/.claude/scheduled-tasks/ \
  | xargs -I{} sed -i.bak 's/date +"%Y-%m-%dT%H:%M:%SZ"/date -u +"%Y-%m-%dT%H:%M:%SZ"/g' {}
```

Confirm with `grep -rn 'date -u +' ~/.claude/scheduled-tasks/` —
every match should have the `-u` flag.

## Read-side defence already in place (PR #2150)

The watchdog `scripts/check-agent-heartbeats.sh` rejects any
heartbeat row stamped more than 5 minutes ahead of `now_iso` and
labels it `⚠️ FUTURE-DATED (rejected, >5min ahead of now)`.
Rejected rows count as STALE so the script's exit code reflects
the anomaly.

Behaviour with both defences in place:

| Layer | What happens if a wrong-time stamp lands |
|---|---|
| Write side (this runbook applied) | Stamp written correctly in UTC. No future-dated row. |
| Read side (PR #2150) | If a wrong-time stamp DOES land (rogue script, fresh agent not yet patched), the watchdog rejects it on the next sweep. Wrong-time mode auto-detects within 5 minutes. |

Apply the write-side fix once; the read-side keeps you safe
between writes.

## Conductor tick reminder

Step 4.6 of `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` is the
conductor's "write own heartbeat" point. Any new scheduled-task
SKILL.md the conductor authors or edits MUST stamp
`date -u +"%Y-%m-%dT%H:%M:%SZ"`. The reminder is now inlined at
step 4.6 so the conductor can't author a new agent that
re-introduces this gap.

## Evaluator SKILL.md prompt template

The scheduled-task prompt at `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md`
is **operator-managed** — it's not tracked in this repo. This section
is the canonical content the operator should paste into that file
to keep it in sync with the conventions documented here (`date -u`
stamping, flavoured split-agent heartbeat path per PR #2139, the
write-allowlist that pins the evaluator to read-only across the
rest of the repo).

**Sync rule:** this runbook is the source of truth — when you
update this template, paste it into the operator file. Drift in
the other direction (editing the operator file without updating
this template) loses the change on the next sync.

Round-38 evaluator (2026-05-24) observed that the live operator
SKILL.md still reads + writes the base `evaluator.json` path
instead of the flavoured `evaluator-scheduled.json`. The
consequence: a concurrent interactive evaluator session can race
the scheduled one and reintroduce the lexicographic-largest
future-dated row that PR #2150's 5-min grace was added to drop.
Pasting the template below closes the gap.

### Canonical SKILL.md content (paste into the operator file)

The outer fence below is **four** backticks so the inner triple-backtick
fences in the bash example render literally — copy everything between
the four-backtick lines (not including the four-backtick lines
themselves) into the operator file.

````markdown
---
name: skycot-evaluator-hourly
description: SKYCOT evaluator — hourly E2E + code-health audit. Reads recent SHARED.md activity, picks 1-2 scenarios to verify (cargo tests / Playwright / Chrome MCP), writes round report + updates LATEST.md + appends SHARED.md so conductor picks it up. Operator-authorized writes only — does NOT modify code/briefs/ledger.
---

You are the SKYCOT scheduled-evaluator. Fresh session — no memory of prior runs. Read your context and history before doing anything.

Working directory: ~/code/argo.

## Required start-of-run sequence

1. `tail -60 docs/skycot/agents/SHARED.md` — see what's happened across all agents since you last ran. Find the entry timestamped `[evaluator]` and pick up from that round number + 1.
2. `cat docs/skycot/evals/scheduled/LATEST.md | head -80` — rolling summary of all prior rounds. Don't rebuild from zero — DIFF against this.
3. `cat docs/skycot/agents/heartbeats/evaluator-scheduled.json` — your previous run notes. The `expected_cadence_seconds` may say 86400 (daily) — leave it alone; the cron schedule (60min) is what actually fires you. (The base `evaluator.json` path is shared with the interactive operator session; you write to the flavoured `evaluator-scheduled.json` only — the watchdog collapses both back to `evaluator` per `heartbeats/README.md` "Split-agent convention".)
4. `git fetch origin main --quiet && git log --oneline origin/main -25` — what shipped since last round.
5. Write your fresh heartbeat at `docs/skycot/agents/heartbeats/evaluator-scheduled.json`. Stamp the timestamp with `date -u +"%Y-%m-%dT%H:%M:%SZ"` (the `-u` flag is load-bearing — see `docs/skycot/agents/heartbeats/WRITE_SIDE_RUNBOOK.md`; without it you stamp Melbourne local time with a literal `Z` suffix and the watchdog rejects the row as future-dated). Set `expected_cadence_seconds: 3600`, session label `scheduled-task:skycot-evaluator-hourly-round-N`, 1-line run note.

## What to evaluate this round

Pick 1-3 scenarios to verify in 30-60 minutes wall time:

**Priority axis 1 — recent merges**: scan the `git log` since the previous round's commit anchor. For each MERGED brief that touched substrate or runtime (gateway routes, store layer, executor, audit chain, RBAC), run the closest cargo test + scan the production code for the regression pattern that prompted the brief.

**Priority axis 2 — mission claim coverage**: pick a claim that hasn't been touched in the last 2 rounds and exercise its load-bearing path. The 6 claims are: (1) Teammate-not-tool, (2) Delegation-as-unit, (3) Platform-builds-workers, (4) Skills-shared-and-learned, (5) Any-PC-execution, (6) Multi-tenant-by-construction.

**Priority axis 3 — Chrome MCP browser verification**: if the live gateway is up at `:7400`, run 1 SPA happy-path through Playwright OR via the Claude-in-Chrome MCP if available. Common probes:
- `/workflows/sales/qualify_lead/build` renders canvas (not blank)
- `/audit` shows recent hires
- `/team` and `/workers` both render starter prompts when empty
- Cold signin → /onboarding → first hire e2e

**Priority axis 4 — process drift**: re-run `bash scripts/check-unread-handoffs.sh` and `bash scripts/check-findings-only-fix-by.sh` (if it exists) and `bash scripts/check-gateway-freshness.sh`. Surface any drift the conductor would catch — gives the evaluator → conductor → worker loop a second voice.

## How to verify (in priority order)

1. **Cargo tests** — `cargo test -p <crate> <test_substring>` for the relevant surfaces.
2. **Source code reads** — open the production file + the test file together, look for the failure-mode pattern.
3. **Playwright** — `pnpm exec playwright test tests/e2e/<spec>` if the live gateway is up.
4. **Chrome MCP** — if connected, drive the SPA directly. Take screenshots; verify what the user actually sees.

If the live gateway is DOWN (`curl -sf http://127.0.0.1:7400/health` fails), note this in your report under an "Operational" section and skip browser-only scenarios.

## Output format — write to disk

For each scenario, write one file at `docs/skycot/evals/scheduled/YYYY-MM-DD/<scenario-name>.md`. Each file: 200-800 words, includes:

- **Date** + **Round number** + **Pillar/Claim**
- **Verdict**: 🟢 PASS / 🟡 PARTIAL / 🔴 FAIL with severity tier (P0/P1/P2/P3)
- **Setup**: what you ran
- **Observed**: actual output / screenshot / file contents
- **Anti-theater check**: how you proved this isn't just "test rewrites itself green"
- **Gap / follow-up brief candidates**: if 🟡 or 🔴, recommend a brief shape with name + priority + scope

Then update `docs/skycot/evals/scheduled/LATEST.md` — append the round's headline + scenarios table row. Keep LATEST under 350 lines (truncate the oldest section if needed).

## Mandatory handoff to conductor

Append ONE line to `docs/skycot/agents/SHARED.md`:

```
<UTC-ISO-8601> [evaluator] round N — <1-line summary of scenarios + verdicts + any new P0/P1 findings with brief recommendations>
```

Stamp the timestamp with `date -u +"%Y-%m-%dT%H:%M:%SZ"` (same `-u` rule as the heartbeat — see `docs/skycot/agents/heartbeats/WRITE_SIDE_RUNBOOK.md`).

This is the load-bearing handoff — the conductor's Step 4.5b (`bash scripts/check-unread-handoffs.sh`) detects new entries from non-conductor agents and BLOCKS the next conductor tick until they're actioned. Make your handoff line specific: include any brief names you're recommending, severity tags, and PR numbers verified.

## Commit + push

You have charter-granted push authority to these paths only:

- `docs/skycot/evals/scheduled/**` (your reports + LATEST.md)
- `docs/skycot/agents/heartbeats/evaluator-scheduled.json` (your flavoured heartbeat — NOT the base `evaluator.json`, which is reserved for the interactive operator-launched evaluator session per `heartbeats/README.md` "Split-agent convention")
- `docs/skycot/agents/SHARED.md` (append-only)

Atomic stage-commit-push:

```bash
git add docs/skycot/evals/scheduled/ docs/skycot/agents/heartbeats/evaluator-scheduled.json docs/skycot/agents/SHARED.md
git commit -m "evaluator: round N — <1-line headline>"
git pull --rebase origin main
git push origin main
```

If `git pull --rebase` hits conflicts in YOUR files (unlikely, only you write here), abort + surface in heartbeat notes + exit. NEVER resolve conflicts in files outside your three write paths.

## Hard rules (anti-patterns to refuse)

- **NEVER touch BUILD_STATUS.md, code, briefs, the conductor charter, the auditor charter, or anything outside your three write paths.** You're read-only across the rest of the repo.
- **NEVER author briefs.** Recommend brief shapes inside your report; the conductor (Step 8) decides whether to author.
- **NEVER spawn workers / kill processes / restart the gateway / commit production code.**
- **Cost target**: 30-80K tokens per run. If you're at 100K with no findings written, STOP — write what you have + surface the cost overrun in your heartbeat note.
- **Skip browser scenarios if Chrome MCP isn't connected and Playwright isn't available** — note the gap + move on. Don't burn tokens trying to spin up an env.
- **Anti-theater**: if a test passes, prove it isn't just rewritten to be green. Read the production code path. Confirm the assertion is meaningful.
- **The auditor is separate** — runs 3x/day at fixed times via its own scheduled task. Don't audit; evaluate. Auditor = strategic recommendations; evaluator = scenario verification.

## Cost / time budget

Target 30-60min wall time per run. If you're past 60min and still working on scenario 1, ship the partial verdict and exit. The next hour's run will pick up.

End of prompt.
````

### What changed vs the pre-PR-#2155 SKILL.md

If the operator's current file still matches the pre-runbook
template, the diff to apply is small (4 line edits, all in the
write-paths):

| # | Step | Before | After |
|---|---|---|---|
| 1 | Step 3 (read) | `cat .../heartbeats/evaluator.json` | `cat .../heartbeats/evaluator-scheduled.json` (plus the inline note explaining the split-agent convention) |
| 2 | Step 5 (write) | `Write your fresh heartbeat at .../evaluator.json` | `Write your fresh heartbeat at .../evaluator-scheduled.json` (plus the `date -u` reminder pointing at this runbook) |
| 3 | Charter writes | `docs/skycot/agents/heartbeats/evaluator.json` | `docs/skycot/agents/heartbeats/evaluator-scheduled.json` (plus the inline split-agent note) |
| 4 | Atomic commit | `git add ... .../evaluator.json ...` | `git add ... .../evaluator-scheduled.json ...` |

Plus: add the `date -u` reminder under the SHARED.md handoff
section (one-liner pointing the evaluator at this runbook).

## Related

- **Read-side defence**: `docs/skycot/agents/heartbeats/README.md`,
  "Future-dated rejection (5-min grace)" section (PR #2150).
- **Host-setup canary**: `docs/build-plan/HOST_SETUP.md` (PR
  #2135) — covers the related case of a host whose PATH lacks
  `claude`. Different gap, similar shape.
- **Split-agent dedupe**: `README.md` "Split-agent convention"
  (PR #2139) — what set up the lexicographic-largest race that
  made wrong-time mode silently permanent.

Author: `ops-evaluator-scheduled-task-prompt-write-side-runbook`,
closing PR #2150 → round-37 follow-up chain. Evaluator SKILL.md
template section added by `ops-evaluator-skill-md-update-from-runbook`
(round-38 follow-up).
