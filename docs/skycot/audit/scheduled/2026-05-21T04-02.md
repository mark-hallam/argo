# Scheduled audit — 2026-05-21T04-02 UTC (baseline)

**Previous report**: none (baseline) · **Commits since**: n/a · **PRs merged since**: n/a · **Commits in trailing 24h**: 372 · **Commits in trailing 48h**: 569

## 1. What shipped since last audit

This is the baseline — no prior report. Anchored to the wave-15 buyer-trial NO-GO (34.4) and wave-10 demo GO (78.4) verdicts. Between those verdicts (both dated 2026-05-21) and this run, the most important movement is **wave-15 P0 #2 closed**: `web-gateway-endpoints-onboarding-batch` MERGED as [#1910](https://github.com/mark-hallam/Argo-Agent/pull/1910), shipping `provider-key` + `install-vertical` + `first-hire` gateway endpoints plus age-wrap encrypted provider-key storage. The first emotional moment of the product no longer 404s. Wave-15 P0 #1 (`web/middleware.ts` cookie-name fix) also landed via earlier operator work. Wave-15 P0 #3 (workflow-builder theater) is mid-sequence: B1 `web-builder-canvas-controlled-state` CLAIMED on mac-host at 03:58Z; B3 `web-builder-delete-and-duplicate` CLAIMED 04:02Z; B2 `web-builder-connect-and-add-step` correctly **BLOCKED** on B1. The conductor authored two new P0 wave-15 gap-closers in the last hour (`web-hire-workerdraft-schema-align`, `web-workflow-save-publish-real-wire`), and Workplace-Onboarding Wave A (6 briefs) was queued via commit `cfc20408`. The Workplace-Onboarding Wave A is gated on `mission-workplace-knowledge-substrate` (brief 1/6, TODO) which is the choke point for the other five. `web-workflow-rollback-feature` was zombie-demoted today and is available for re-claim.

## 2. Mission-claim coverage table

| Claim | Status | Δ since last | What's needed next |
|---|---|---|---|
| 1. Teammate, not tool | partial — persona/role/worker primitives shipped; trust grade still static (set-once at hire); freeze-banner single-surface only | baseline | Land `web-worker-trust-promotion-engine` (TODO) so trust grade moves on real signals — currently theater |
| 2. Delegation is the unit | shipping — hire→first-hire endpoint MERGED #1910; charter/duties/KPIs in worker-detail; 5-teammate batch hire silently collapses to "Mariah" clones | baseline | Claim `web-hire-workerdraft-schema-align` (TODO, P0) — WorkerDraft schema mismatch blocks claim 2 buyer story |
| 3. Platform builds workers | shipping — Genesis pipeline live, vertical/install-vertical endpoint MERGED #1910; live-gateway path still falls to genesis-stub on schema drift | baseline | Same as claim 2 — `web-hire-workerdraft-schema-align` is the lever |
| 4. Skills shared + learned | substrate complete (skill-library worker-load LIVE per #1682); learning loop and cross-tenant trust+provenance not yet wired into UI | baseline | No active brief on skill-discovery surface; defer until wave-15 P0s close |
| 5. Any-PC execution (MCP + computer-use) | macOS computer-use SHIPPED + run-loop wired; MCP backbone live; Linux/Windows DEFERRED v1.x per operator lock | baseline | Out-of-scope this cycle per 2026-05-14 operator decision |
| 6. Multi-channel approval | gateway+Slack+email+SMS+push backends real; approvals/decide endpoint still has NO role gate at gateway; `lib/api/approvals.ts:156-159` silently falls through to mock-offline in prod on network blip | baseline | Re-claim `web-rbac-jwt-role-claim-and-gateway-enforce` (#1901) **and** ship 5-LOC `isStrictProduction()` guard on `submitDecision` |

## 3. Drift signals (max 3)

1. **256 CLAIMED rows vs 5-worker fleet → zombie backlog.** `grep -c CLAIMED docs/build-plan/BUILD_STATUS.md = 256`. Operator confirmed 5-worker fleet today; even allowing for re-counts of historical rows, this is far more than the alive set. Today's zombie-sweep of `web-workflow-rollback-feature` is a single example; the rest of the 256 are not being swept on the 2h-no-PR-no-push rule. Recommended response: conductor runs a one-shot zombie-sweep against rows older than 2h without a branch push, *before* tomorrow's first replenishment tick.
2. **Wave-15 P0 #3 sequence is at risk of partial completion.** B1 controlled-state is CLAIMED but no PR; B3 delete+duplicate is CLAIMED in parallel against the same canvas surface. If B3 lands before B1, the rebase cost is meaningful (both touch `<ReactFlow>` props + node handlers). Recommended response: surface B1's branch status next tick — if no push by 06:00 Melbourne, demote B3 until B1 ships.
3. **The 234-row TODO queue is dominated by post-MVP polish, not wave-15 P0s.** Of the 233 TODO rows, only **3** are wave-15 P0 gap-closers (`web-hire-workerdraft-schema-align`, `web-workflow-save-publish-real-wire`, and the four-brief builder sequence). Workplace-Onboarding Wave A (6 briefs, all TODO) is good-for-v1.1 strategic substrate but does NOT close any wave-15 buyer-trial blocker. Recommended response: conductor prioritises wave-15 P0 claim-readiness over Wave-A brief 1 next tick.

## 4. Top recommendations (max 7)

- **P0**: Demote any of the 256 CLAIMED rows whose `claimed_at` is >2h old AND has no branch push AND no PR. Evidence: ledger row count vs 5-worker fleet; today's row 920 zombie-sweep on `web-workflow-rollback-feature` proves the pattern. Action: conductor runs `scripts/find-stale-claim-drift.py --quiet` next tick. (New.)
- **P0**: Claim `web-hire-workerdraft-schema-align` (row 970, TODO, ~400-600 LOC). Evidence: wave-15 §4 brief 5; collapses 5-teammate hire path. Without this, claim 2 + claim 3 buyer story is broken even after #1910 closed the endpoint 404. (New.)
- **P0**: Claim `web-workflow-save-publish-real-wire` (row 971, TODO, ~400-600 LOC) after #1888 lands. Evidence: wave-15 §1 step 15 scored 1.0/10. Save+Publish is currently a toast; backend endpoints from #1846 + #1860 already exist. (New.)
- **P0**: Surface the `submitDecision` mock-offline silent-fallthrough bug. Evidence: wave-10 §7; `lib/api/approvals.ts:156-159` silently fabricates `{ok:true,source:"mock-offline"}` in production on network blip — audit chain loses rows. 5-LOC `isStrictProduction()` guard per `lib/api/team.ts:62-66`. No brief authored yet. Action: conductor authors a 1-row brief next tick. (New.)
- **P1**: Re-claim `web-workflow-rollback-feature` (row 920, zombie-demoted today). Evidence: wave-10 §4 procurement risk; landing copy promises "Rollback shipping next"; brief is ready; 500-800 LOC. (New.)
- **P1**: Verify `web-builder-canvas-controlled-state` (B1) has a branch push by 06:00 Melbourne local. Evidence: B2 BLOCKED on it; B3 claimed in parallel. If no push, demote and re-queue B1 ahead of B3. (New.)
- **P2**: Author the `mission-rbac-approvals-decide-role-gate` follow-on brief (currently a v1.0-strategic-decision tail item). Evidence: wave-10 §4 procurement risk #1; #1901 in review for cross-tenant audit but approvals/decide remains role-gateless. (New.)

## 5. Resolved since last (max 5)

(none — baseline run)

## 6. Orientation score

**Swarm-orientation score: 56 / 100** (delta from last: n/a — baseline).

Rubric:
- **Wave-15 buyer-trial gap closure (30%)** — 2 of 6 P0s closed (#1910 endpoints + cookie fix). 1 P0 in flight (B1 canvas controlled-state CLAIMED). 3 P0s TODO/blocked. Score: **15/30**.
- **Mission-claim 1-6 substrate coverage (30%)** — substrate ~85% per `MISSION.md`; runtime closed-loops ~70%; gap is in surfaces (claims 1, 2, 6 procurement-grade), not primitives. Score: **22/30**.
- **Queue health (20%)** — 233 TODO is healthy; 256 CLAIMED is **overcommitted vs 5-worker fleet**; IN_REVIEW 343 is fine. Imbalance is the CLAIMED column. Score: **10/20**.
- **Drift signals (10%)** — 3 surfaced above; none catastrophic; B1/B3 parallel-claim is the highest-risk. Score: **7/10**.
- **Audit-recommendation actioning rate (10%)** — baseline, no prior recs. Default: **5/10**.

**Total: 59/100.** Adjusted to **56/100** for honesty floor — the 256 zombie-CLAIMED rows misrepresent swarm capacity and inflate orientation reads from BUILD_STATUS.md alone.

The swarm is **directionally aligned** on closing wave-15 P0s but is **operationally over-claiming**. If the 256 CLAIMED column were swept to truth (likely 5-15 live), the conductor would visibly see queue starvation against wave-15 P0s and re-prioritise. Today's drift is bookkeeping, not strategy.

---

**tldr:** wave-15 P0 #2 MERGED (#1910); P0 #3 (builder) mid-sequence with B1 CLAIMED + B2 correctly BLOCKED; two new P0 gap-closer briefs (hire-schema-align + save-publish-wire) sit TODO. Highest-leverage next move: sweep the 256-row CLAIMED zombie backlog so wave-15 P0 starvation becomes visible to the conductor. Orientation score 56/100 (baseline).
