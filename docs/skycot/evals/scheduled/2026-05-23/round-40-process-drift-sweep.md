# Round 40 — Process drift sweep

- **Date**: 2026-05-23 (real UTC; Melbourne local 2026-05-24)
- **Round**: 40 (scheduled-task hourly fire, read-only)
- **Verdict**: 🟢 mostly clean, 🟡 P1 heartbeat-WRITE-side gap 9th consecutive observation, 🟡 P2 workspace-clippy bypass pattern surfaced

## Setup — gate scripts run

```bash
bash scripts/check-findings-only-fix-by.sh
bash scripts/check-unread-handoffs.sh
bash scripts/check-gateway-freshness.sh
bash scripts/check-todo-aging.sh    # added round-38 by PR #2152 sibling
```

## Observed — findings-only fix-by tracker (5/5 within budget)

```
🟢 eval-cron-duty-fires-agent-runner-end-to-end           within budget (2026-05-30, 7 days remaining)
🟢 eval-workflow-execution-runs-real-step                 within budget (2026-05-30, 7 days remaining)
🟢 mission-hire-audit-payload-shape-validation            within budget (2026-05-30, 7 days remaining)
🟢 mission-cu-macos-cross-tenant-capability-test          within budget (2026-05-30, 7 days remaining)
🟢 mission-cross-tenant-test-audit-export                 within budget (2026-05-25, 2 days remaining)

Tally: 5 within-budget · 0 due-today · 0 overdue · 0 informational · 5 total.
```

**Flag for next round (41)**: `mission-cross-tenant-test-audit-export` deadline is 2026-05-25 — only 2 calendar-days away. If the row is still TODO at tick-122, downgrade should kick in.

## Observed — unread-handoffs gate (clean)

```
✅ No unread handoffs from other agents (cutoff 2026-05-24T17:15:00Z).
```

This is a CHANGE from rounds 37-39 (which all showed 26 stale conductor entries past the cutoff). Some recent conductor tick — likely tick-119 or tick-120 — re-aligned the cutoff window. Net result: the conductor → worker → evaluator handoff loop is currently uncluttered.

## Observed — TODO-aging gate (3/3 within budget)

```
🟢 mission-gateway-session-export-tenant-bind             within budget (17h since 2026-05-23T00:00Z)
🟢 demo-ops-add-magic-link-bypass-for-recording           within budget (17h since 2026-05-23T00:00Z)
🟢 docs-demo-runbook-add-skycot-db-key-section            within budget (17h since 2026-05-23T00:00Z)

Tally: 3 within-budget · 0 aging · 0 overdue · 3 total.
```

The 24h aging threshold has not yet fired for any of these 3 P-tagged TODO rows. Next observation point: ~7h from now (tick-122-ish), the 24h budget would tip on the oldest row.

## Observed — gateway freshness (UNREACHABLE, informational)

```
STATUS: UNREACHABLE  (no response from http://127.0.0.1:7400/health — gateway not running?)
```

Informational only — the scheduled-task evaluator does not start the gateway. No SPA / Playwright scenarios attempted this round (per charter — skip browser scenarios if gateway is down).

## Heartbeat WRITE-side gap — 9th consecutive observation 🟡 P1

This round's heartbeat stamp at `docs/skycot/agents/heartbeats/evaluator-scheduled.json` is correctly real-UTC (`2026-05-23T17:27:00Z`) — verified by `date -u +%Y-%m-%dT%H:%M:%SZ` returning `2026-05-23T17:27:08Z` at run-start.

But the SHARED.md history shows the pattern persists across operator-interactive evaluator sessions:

- Round 38 (`13:27:32Z` in SHARED) = Melbourne local `13:27` AEST stamped as Z, ~10h ahead of real UTC at the time.
- Round 35 (`12:30:00Z` in SHARED) = also Melbourne local stamped as Z.
- Round 39 (`16:30:00Z` in SHARED) = real UTC; this scheduled-task session prompted `date -u` and used it.

PR #2150 (watchdog READ-side defence: reject future-dated rows with `⚠ FUTURE-DATED (rejected, >5min ahead of now)`) is working perfectly — verified by reading the conductor's tick-118 line in SHARED.

**WRITE-side gap remains operator-actionable**: the SKILL.md prompts at `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` and the interactive-evaluator launch surface still don't enforce `date -u`. Round-39 already flagged `ops-scheduled-task-prompt-add-explicit-date-u-instruction` as a P1, ~5-10 LOC × 5 SKILL.md files. **5 rounds open. Operator-actionable (outside repo).** Re-surface for visibility.

## Observation new this round — workspace-clippy bypass pattern 🟡 P2

From PR #2159's boundary note: pre-existing clippy violations in `claim_4_mvp_row_5_pending_review_queue.rs` (`doc list item without indentation`, `assertion has a constant value`) and `claim_4_mvp_row_6_operator_approve_emits_audit.rs` (`unnecessary closure used to substitute value for Option::None`) are forcing every recent admin-merge to use `--skip-clippy`. PR #2159 closed the LOC-budget bypass that was running; these clippy bypasses remain.

This is the same anti-pattern: a CI gate being routinely bypassed because of pre-existing dirt that nobody owns. Recommend brief:

- **`mission-clippy-fix-claim-4-row-5-and-row-6-warnings`** (P2, ~10-20 LOC, ~10 min effort) — fix the three flagged clippy violations so gate-5 (workspace clippy) is restored to passing. Single mechanical fix; no scope creep.

## Gap / follow-up brief candidates

**Re-surfaced (not new from this round)**:

- `ops-scheduled-task-prompt-add-explicit-date-u-instruction` (P1, ~5-10 LOC × 5 SKILL.md files, **operator-actionable, outside repo**) — 9th-consecutive-round observation; round-39 named it; **5 rounds open**.
- `eval-gateway-uuid-bound-routes-tenant-ownership-sweep` (P1, ~150-200 LOC) — round-33 first named, round-35 / round-37 / round-39 re-surfaced, **6th consecutive tick unauthored despite tick-118 noting brief file now exists at line 1195**. Sweep would catch every gateway route that binds on UUID alone without a tenant ownership pre-flight — the same pattern that landed `mission-approval-decide-request-tenant-pinned` (#2146) and `mission-gateway-session-export-tenant-bind` (TODO).
- `mission-agent-runner-wire-process-registry-per-session` (P1) — F1 sibling of the #2117 triad; F2+F3 closed in round 38, F1 still open.
- `mission-cross-tenant-test-audit-export` (P0, fix-by 2026-05-25 in 2 days) — close to deadline.

**New this round**:

- `mission-clippy-fix-claim-4-row-5-and-row-6-warnings` (P2, ~10-20 LOC) — close the workspace-clippy `--skip-clippy` bypass pattern documented in PR #2159's boundary note.

## Headline

Process gates **CLEAN** this round (5/5 fix-by + 3/3 todo-aging + 0 unread handoffs). Heartbeat WRITE-side gap is the only persistent P1 across rounds; PR #2150's READ-side defence absorbs the damage but the root-cause prompt fix is operator-actionable and still open.
