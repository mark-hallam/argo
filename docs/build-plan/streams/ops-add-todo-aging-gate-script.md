# ops-add-todo-aging-gate-script

**Authored against:** `origin/main` 6d148f0e1585b73b9527ef3ed8a0201d27eb4aca on 2026-05-23.
**Validate before claim:** `bash scripts/check-brief-freshness.sh ops-add-todo-aging-gate-script`.

## Why this brief exists

Scheduled audit `docs/skycot/audit/scheduled/2026-05-23T09-40.md` §4 P1 #4 (also surfaced by round-31 evaluator finding):

> The cu-macos P0 went **17h+ unfixed** during the mac-host doom-loop because `scripts/check-findings-only-fix-by.sh` by design only walks MERGED FINDINGS-ONLY rows, not TODO rows. A companion `scripts/check-todo-aging.sh` walking TODO rows for P0 tokens and flagging any P0 unfixed >24h would have escalated the doom-loop in <1h. Acknowledged at tick-101/104 but not authored.

Goal: catch the next "P0 TODO that nobody touches" within hours, not days.

## Acceptance criteria

1. New script at `scripts/check-todo-aging.sh` (~50-100 LOC, shell + python or pure bash). Walks `docs/build-plan/BUILD_STATUS.md` for rows where `status` is `TODO`, parses the row note for P0 tokens (`P0`, `🔴 P0`, `🚨 P0`, `OPERATOR-LOCKED`), and emits:
   - `🟢 <brief> within budget (<N> hours since claimed/authored)` if <24h
   - `🟠 <brief> aging (<N> hours)` if 24h ≤ age < 48h
   - `🔴 <brief> OVERDUE (<N> hours)` if ≥48h
2. Determine "age" from the row note's most recent `2026-MM-DDTHH:MMZ` timestamp (any of: `NEW <date>`, `claimed at <date>`, `RESTORED <date>`). If none found, fall back to git-log first-introduction date of the row.
3. Tally line at end (matches the FINDINGS-only script shape): `Tally: N within-budget · N aging · N overdue · N total.`
4. Exit 0 always (informational; do not block CI). Conductor surfaces in tick summary like the gateway-freshness + findings-only scripts.
5. Conductor adds a one-line invocation to `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` step 4.8 area (or sibling 4.9), e.g.: `(4.9) bash scripts/check-todo-aging.sh — surface aging P0 TODO rows`.

## Parallel-surface declarations

- New script, no existing parallel surface. `scripts/check-findings-only-fix-by.sh` is the shape template — mirror its output format.
- No new pub Rust API.

## May-touch

- `scripts/check-todo-aging.sh` (new)
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` (one-line addition for conductor invocation)

## Boundary

- Do NOT modify `BUILD_STATUS.md` parsing in any other script.
- Do NOT add a CI hook — purely a conductor-surfaced informational gate.

## Sizing

~50-100 LOC + 1 docs line. Fast-mode candidate.

**🟡 P1 OPS (escalation gate)** — turns "unfixed for 17h+ during outage" into "flagged within an hour".
