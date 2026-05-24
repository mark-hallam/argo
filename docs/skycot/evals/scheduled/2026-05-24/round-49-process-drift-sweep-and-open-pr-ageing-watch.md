# Round 49 · Process drift + open-PR ageing watch

- **Date**: 2026-05-24 (UTC) · Melbourne 2026-05-25
- **Round**: 49
- **Pillar**: Process integrity (the conductor's other voice)
- **Verdict**: 🟢 **CLEAN** across all 3 lint gates; 🟡 **6 PRs ageing 7-8h** (no v1.0 demo-blockers, no action required this round)

## Drift sweeps

```bash
bash scripts/check-unread-handoffs.sh        # → ✅ No unread handoffs from other agents (cutoff 2026-05-25T19:08:10Z).
bash scripts/check-findings-only-fix-by.sh   # → 🟢×4 within budget (all 2026-05-30, 6d remaining); Tally: 4 within-budget · 0 due-today · 0 overdue
bash scripts/check-gateway-freshness.sh      # → gateway_schema=v80 codebase_latest=v80 drift=0 | STATUS: FRESH
```

All three gates GREEN. Findings-only watch:

- `eval-cron-duty-fires-agent-runner-end-to-end` (claim-2 LIVE coverage)
- `eval-workflow-execution-runs-real-step` (claim-3 LIVE coverage)
- `mission-hire-audit-payload-shape-validation` (claim-2 audit-chain hygiene)
- `mission-cu-macos-cross-tenant-capability-test` (claim-5 + claim-6 intersection)

No P0 audit carries action-needed this round.

## Open-PR ageing watch (>7h since createdAt)

```bash
gh pr list --state open --limit 50 --json number,title,createdAt \
  --jq 'map(select(.createdAt < "2026-05-24T12:00:00Z"))'
```

| # | Title | Age (h) | Track | Risk |
|---|---|---|---|---|
| 2205 | docs — demos — consolidate v1 demo runbook | 7.9 | Demo runbook | Low — doc-only, holds during merge waves |
| 2206 | P3 — Skills — dark-mode search chrome + mobile | 8.0 | UI polish | Low — explicitly P3 |
| 2207 | Fix SPA error pages and audit redactor | 7.7 | UX hardening | Medium — touches audit-redact path, conductor should review next tick |
| 2208 | mission — voice approvals — audit DTMF replies | 7.7 | Track P (P-pitch-agent) | Medium — Track P substrate, in operator-locked v1.0 MVP scope per memory |
| 2209 | Add provider-agnostic voice I/O crate | 7.7 | Track P (P-03) | **Medium-High — Track P bottleneck per r47/r48 notes** |
| 2210 | Phase A — CLI JSON contract — fill list/show/status gaps | 7.9 | Backend invariant (JSON output for CLI list/show/status) | Medium — affects CLAUDE.md "JSON output for CLI" invariant |

None are demo-MP4 ship-gates (#2247/#2241/#2244 are the demo blockers and all sit in r47-r49 PRE-MERGE PASS verified). T19-08 conductor tick already flagged the round-47+48+49 PRE-MERGE PASSes as next-tick merge cluster candidates, so demo-path is unblocked.

**T19-08 conductor process clarification (important):** prior T17-33 + T18-06 ticks deferred all merges citing operator 'no cargo run by conductor' amendment, but per `codex-handoff-prompt.md:20-28` that override is **CODEX-conductor-specific**. Claude-conductor sessions follow normal PROTOCOL_LOOP_PROMPT.md Step 5 (diff-shape gate + conductor-pre-merge-gate.sh + admin-merge). The 14 PRs that closed/merged between T18-06 and T19-08 (open-PR count 44 → 30) reflect this clarification being applied. Demo-blocker merge cluster (#2241/#2244/#2247/#2248) should land next claude tick.

## Re-surface unactioned (carries from r47/r48)

- 🔴 **`ops-evaluator-skill-md-update-from-runbook` (P2, operator-actionable on non-repo file)** — **14th consecutive round of SHARED.md timestamp-drift** in worker `git reset --hard` clobbers; same heartbeat re-write workaround required this round. Operator action on `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` to enforce `date -u` stamps would close this drift class. Outside evaluator's three write paths.
- 🟡 `ops-pre-merge-gate-fix-test-failures-not-just-clippy` (P1) — still BLOCKED on missing stream file per r47.
- 🟡 `mission-agent-runner-wire-process-registry-per-session` (P1) — F1 sibling of #2117 triad, open since r41.
- 🟡 `fix-web-inherited-red-test-cluster-2241-baseline` (P1, surfaced r47) — 6 pre-existing RED web tests pollute pre-merge gate floor for #2241/#2242/#2243 wave.

## Brief candidates this round

None new from process drift. Two carried from r48 backed by live-probe evidence in [round-49-gateway-vertical-discovery-sibling-gap-confirm.md](round-49-gateway-vertical-discovery-sibling-gap-confirm.md):

- (a) `fix-web-verticals-route-cors-preflight-symmetric-fix` (P1-if-demo-MP4-touches, else P2)
- (b) `mission-gateway-add-api-v1-verticals-cors-mounted-discovery` (P2)

## Track P / v1.0 demo gate snapshot

Track P v1.0 MVP scope (operator-amended 2026-05-24) requires all 8 P-briefs MERGED before demo MP4 records:

| P-brief | PR | Status r49 |
|---|---|---|
| P-01 mission-pitch-agent-* | #2214 | IN_REVIEW (per r47/r48; recheck mergeable) |
| P-02 | — | — |
| P-03 (voice I/O) | #2209 | **OPEN 7.7h (ageing watch)** |
| P-04..P-08 | — | — |

#2208 (voice approvals DTMF) also Track P. T19-08 conductor flagged next-tick merge cluster — Track P should fold in.
