# ops-conductor-ghost-stamped-tick-detection — detect conductor SHARED.md entries that DIDN'T refresh conductor.json heartbeat (θ-r70 ghost-tick pattern)

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `3a6d7c376` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-conductor-ghost-stamped-tick-detection`.

## Why this is P2 (θ-r70 — NEW process drift, 1st-round)

Evaluator round-70 caught a new failure mode of the conductor protocol it labelled θ-r70: the scheduled conductor tick at T19-08 (2026-05-25T19:08:10Z) **appended an entry to `docs/skycot/agents/SHARED.md`** with state numbers that were factually wrong AT THE TIME OF WRITING (claimed Open PRs=30 vs gh's actual 6; "gateway FRESH v80=v80" vs live 503; "0 unread handoffs" vs 4 unread; "commits FF'd to ebcddcb3e" vs actual HEAD 2fdb14a9a) **AND did NOT update `docs/skycot/agents/heartbeats/conductor.json`** (`last_run_utc` stayed at 14:10Z, four ticks behind).

The follow-up scheduled tick at T19-30 read this T19-08 SHARED.md line, did not detect the divergence, and marked T19-08 as "informational/converged" — propagating bad state forward. Evaluator round-70 only caught it via independent `gh pr list` + `curl /health` spot-checks.

Root cause hypothesis: the SHARED.md append happened, but the conductor.json write either crashed, was skipped, or never executed (Step 4.6 of `PROTOCOL_LOOP_PROMPT.md` lists them as sequential — if the script aborts between them, you get exactly this signature). The previously-authored `ops-conductor-cutoff-refresh-on-evaluator-handoff-cycle` brief (PR #2337) closes the **conditional-write** case where the cutoff is left frozen; this brief closes the **divergence-detection** case where the cutoff and the SHARED.md line don't match.

Round-70 source: `docs/skycot/evals/scheduled/2026-05-25/round-70-process-drift-sweep.md` (θ-r70 finding).

## Scope

One-part detection script + wire-up.

**Add `scripts/check-conductor-ghost-stamps.sh`** (~40-60 LOC bash):

1. Read the last N (default 10) `[conductor]` entries from `docs/skycot/agents/SHARED.md`. Each starts with an ISO-8601 timestamp.
2. For each, parse the entry's `agent-stamped` timestamp from the leading `YYYY-MM-DDTHH:MM:SSZ` token at the line start.
3. Compare against `docs/skycot/agents/heartbeats/conductor.json`'s `last_run_utc` AS-OF that line. The contract is: every `[conductor]` SHARED.md entry MUST be matched by a `last_run_utc` write within ±5 minutes (the same tick's Step 4.6).
4. To check "AS-OF that line", read `git log -p --follow docs/skycot/agents/heartbeats/conductor.json` and grep for the closest `last_run_utc` value to the SHARED.md timestamp. If no `conductor.json` write occurred within ±5min, flag the SHARED.md line as a GHOST STAMP.
5. Exit `0` if no ghost stamps found in the last N entries. Exit `1` (and print each ghost) if any.
6. Wire the check into `scripts/check-unread-handoffs.sh` as a sibling step that runs after the existing unread scan — print a `⚠️ ghost-stamped conductor entries detected (N)` line per finding, but do NOT block the tick. The conductor should still proceed; this is a surveillance signal, not a hard gate (the operator decides whether to escalate to a process change).

**Not in scope**: retroactively fixing past ghost-stamped lines, modifying the SHARED.md format, changing the heartbeat schema, blocking ticks on detection.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `scripts/check-conductor-ghost-stamps.sh` — new file, the detector.
- `scripts/check-unread-handoffs.sh` — append a non-blocking call to the new script.
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — one-line note under Step 4.6 mentioning the new check.

## Files you must NOT touch

- `docs/skycot/agents/SHARED.md` — read-only for this brief.
- `docs/skycot/agents/heartbeats/conductor.json` — read-only for this brief.
- Any file under `crates/`.

## Acceptance criteria

- [ ] `bash scripts/check-conductor-ghost-stamps.sh` exits `0` on a clean repo (last 10 conductor entries all match a heartbeat write within ±5min).
- [ ] Fixture smoke: create a temp SHARED.md with a known-ghost line + a known-good line, point the script at it via env var, confirm it exits `1` and prints only the ghost.
- [ ] `bash scripts/check-unread-handoffs.sh` keeps exit code `1` when handoffs are unread (existing contract), and surfaces a ghost-stamp WARN line when present (additive).
- [ ] Run against the **known-ghost T19-08 entry** in current SHARED.md and confirm it flags. (This is the load-bearing acceptance signal — if it doesn't catch T19-08, the detector is wrong.)
- [ ] BUILD_STATUS.md updated with PR number on flip to IN_REVIEW.

## Branch + PR

- Branch: `claude-ops-conductor-ghost-stamped-tick-detection` or `codex-ops-conductor-ghost-stamped-tick-detection`.
- PR title: `ops: detect conductor SHARED.md entries that didn't refresh heartbeat — θ-r70 ghost-tick surveillance (ops-conductor-ghost-stamped-tick-detection)`.

## Notes for the implementer

- Bash-only, ~40-60 LOC. Fast-mode candidate.
- The trickiest part is "as-of that line" comparison — easiest path: walk `git log --reverse --pretty=format:%H -- docs/skycot/agents/heartbeats/conductor.json`, for each commit `git show <sha>:docs/skycot/agents/heartbeats/conductor.json | jq -r .last_run_utc`, and find the conductor.json snapshot whose timestamp is closest to the SHARED.md line's stamp. If the closest one is >5min away (in either direction) AND no commit was made within ±5min, it's a ghost.
- Alternative simpler path that catches the T19-08 case: just check that for every `[conductor]` line in SHARED.md, there exists a `git log` commit touching `conductor.json` within ±10min of the line's timestamp. If not, ghost. This sidesteps the JSON-value comparison entirely.
- The detector is INTENTIONALLY non-blocking — false positives during interactive operator sessions (where the conductor.json may not be committed for a while) shouldn't break ticks.
- Reference: `docs/skycot/evals/scheduled/2026-05-25/round-70-process-drift-sweep.md` for the θ-r70 finding.
- Companion to PR #2337 (`ops-conductor-cutoff-refresh-on-evaluator-handoff-cycle`): #2337 fixes the **cause** (conditional cutoff-write); this brief adds the **detector** so a future regression of the same pattern surfaces within one tick.
