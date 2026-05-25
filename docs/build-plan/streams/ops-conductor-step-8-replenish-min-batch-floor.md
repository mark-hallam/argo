# ops-conductor-step-8-replenish-min-batch-floor — enforce min 5 briefs per Step 8 replenishment

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-conductor-step-8-replenish-min-batch-floor`.

## Why this is P2 (root cause of T22-07 NO_CLAIMABLE outage)

Conductor protocol Step 8 says "author 3-5 new briefs if queue ≤25", but recent ticks (T20-08, T21-06, T22-07) have authored only 0-2 briefs each. Result: queue oscillates between 0 and 2, slot-2 sits in NO_CLAIMABLE for hours, watchdog flags streak=6 degraded. The protocol prompt is too soft — "3-5" is interpreted as "as few as 1 is OK".

## Scope

1. **Update `docs/build-plan/PROTOCOL_LOOP_PROMPT.md`**: change Step 8 from "author 3-5 new briefs" to "author MIN 5 (target 5-10) new briefs". Add an explicit clause: "If you cannot find 5 candidates after mining the priority sources (gap manifest, MISSION_BACKLOG, PRODUCT_ROADMAP, PR-body deferrals, git log -100), then mine NOT-AUDITED crates from the gap manifest and author `eval-<crate>-surface` briefs — these are always authorisable and always useful."
2. **Add a tick-summary assertion**: if the tick ended with claude_or_codex_todo < 5 AND Step 8 was not run, the tick summary must explicitly say "Step 8 SKIPPED — reason: …". No silent skip.
3. **Document the rule in `docs/build-plan/CONTRIBUTING_AGENT.md`** under the conductor section.

## Files you may touch

- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md`.
- `docs/build-plan/CONTRIBUTING_AGENT.md`.

## Files you must NOT touch

- Anything under `crates/`.
- `scripts/conductor-pre-merge-gate.sh`.

## Acceptance criteria

- [ ] Step 8 text in `PROTOCOL_LOOP_PROMPT.md` says "MIN 5 (target 5-10)" with the explicit fallback clause.
- [ ] Tick-summary assertion clause added.
- [ ] `CONTRIBUTING_AGENT.md` documents the rule.
- [ ] Diff is docs-only.

## Branch + PR

- Branch: `claude-ops-conductor-step-8-replenish-min-batch-floor` or `codex-…`.
- PR title: `ops(conductor): floor Step 8 replenishment at 5 briefs per tick (ops-conductor-step-8-replenish-min-batch-floor)`.

## Notes

- Docs-only. Fast-mode candidate.
- This brief is the meta-fix for the very outage that caused this batch of 20 to be authored.
