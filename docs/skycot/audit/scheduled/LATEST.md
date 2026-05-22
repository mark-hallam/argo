# Scheduled audit — 2026-05-22T03-39 UTC

**Previous report**: 2026-05-21T21-40 · **Interval**: ~6h · **Commits since**: 98 (incl. 21 PR merges) · **PRs merged since**: 21 (#1988 – #2009) · **Open PRs right now**: 8 (#2010 – #2017, all newly opened, mergeability UNKNOWN)

## 1. What shipped since last audit

Strong tactical interval — 21 PRs MERGED in 6h — but the dominant story is a new strategic input: the **evaluator agent fired its bootstrap run for the first time** (commit `90fb64111`) and surfaced **4 P0 production bugs** that block the v1.0 demo MP4 ship-gate. Three of those landed either pre-interval (#1901 cookies-in-client-bundle) or this interval (#1988 `;`-in-SQL-comment breaks fresh-DB migrate). The evaluator did what the swarm was missing — runtime ground-truth against live gateway and cold SPA start. Conductor-side wins: wave-15 phase-1 builder substrate **finally COMPLETE** with #2003 save-button + #2008 publish-button MERGED. All **3 cross-tenant follow-up briefs** from the previous §5 P2 MERGED (#1989 gateway-lookups, #1991 prepared-statements, #1992 audit/retros tenant-scope) — the #1983 P1 data-exposure is now closed end-to-end. The **🚨 ESCALATION on `mission-workplace-knowledge-substrate` was actioned**: commit `bb9df824b` promoted to **P0 OPERATOR-LOCKED** with explicit "NEXT CLAUDE WORKER MUST CLAIM THIS" direction. Auth + responsive + audit-timeline also shipped (#2001 decider-role, #1999 hamburger, #2000 font-floor, #2002 hire-banner). 8 in-review PRs (#2010–#2017) cluster on workflow builder polish — claim-2 surface convergence.

## 2. Mission-claim coverage table

| Claim | Status | Δ since last | What's needed next |
|---|---|---|---|
| 1. Teammate, not tool | hire surface stable; #2002 loud failure banner when Genesis falls to stub MERGED | +1 quality | Evaluator P0-D — audit feed must surface `WorkerHired` (currently invisible to operator) |
| 2. Delegation is the unit | **save-button #2003 + publish-button #2008 MERGED** = wave-15 phase-1 finally COMPLETE; concurrent-edit-banner CLAIMED + 5 builder polish PRs in review | **+3 substantial** | Evaluator P0-A blocks the SPA route the demo MP4 walks through — fix before recording |
| 3. Platform builds workers | #2007 `/verticals` marketplace listing + detail MERGED; #2013 builder LOC-extract-v2 in review | +1 substantial | Live gateway is 92h stale at schema v70 vs code v76 — workflow rollback/sub-workflow/sticky-note routes are not actually in the running binary (evaluator P0-C) |
| 4. Skills shared + learned | `mission-workplace-knowledge-substrate` **promoted to 🚨 P0 OPERATOR-LOCKED** but still TODO; cross-tenant skill test (#5) is BLOCKED by P0-B fresh-DB migration crash | 0 substrate, +1 governance | Worker pickup of the substrate brief is the next single highest-leverage move on this claim |
| 5. Any-PC execution | unchanged macOS-only | held | Out of scope this cycle (operator-locked v1 decision) |
| 6. Multi-channel approval | #2001 `decider_role` on audit drill-in timeline MERGED — closes a prior-audit P3 | +1 surface | Watch closure of decider-role end-to-end test coverage |

## 3. Drift signals (max 3)

1. **🔴 Evaluator P0-B: PR #1988 introduced a fresh-install regression.** Added `;` inside `-- ` SQL comments across 48 migration files; `split_terminator(';')` in `crates/argo-store/src/migrations/v43.rs` (and v47, v51) hands raw English to SQLite and crashes any boot-from-zero gateway at v43. **Every operator who installs SKYCOT today will see the gateway refuse to start.** Highest-severity new regression. See §4 P0 #1.

2. **🔴 Evaluator P0-A: pre-existing live-bundle bug.** `web/auth.config.ts:3` imports `cookies` from `next/headers` — drags a server-only API into Client Component bundles. **5 of 6 in-product routes return HTTP 500 on cold dev start.** Landed in #1901 (pre-interval). Swarm has been shipping builder/audit surfaces on top of routes that 500 on cold start; conductor smoke gate doesn't cold-start the SPA. See §4 P0 #2.

3. **🟡 Evaluator P0-C: shipping ≠ deployed.** Running gateway PID 28198 reports `schema_version: 70` vs code v76. ~30 PRs of new functionality (workflow rollback, sticky-notes, sub-workflows, http+code nodes, hire-banner) **are not in the binary**. Merge-then-flip-ledger is shipping faster than the binary restarts. Even after P0 #1 + #2 land, demo MP4 path 404s on new workflow routes. See §4 P0 #4.

## 4. Top recommendations (max 7)

- **🔴 P0 (new — highest)**: Author **`claude-migration-comment-aware-splitter`** next tick. Evidence: evaluator reports `skill-cross-tenant-leak-blocked-by-migration-bug.md` + `workflow-build-save-publish-rollback.md`. **One fix unblocks 2 currently-FAIL smoke scenarios AND every fresh-install path.** ≤200 LOC; strip `-- ...\n` lines before `split_terminator(';')` in v43/v47/v51; add a one-shot `;-in-comment` unit test. (New.)

- **🔴 P0 (new)**: Author **`web-auth-config-server-only-split`**. Evidence: evaluator `spa-cookies-import-app-routes-500.md`. Move `cookies()` out of `auth.config.ts` into a server-only sibling. **Demo MP4 cannot be recorded against the current SPA without this.** (New.)

- **🚨 P0 (4th-consecutive — ACTIONED, awaiting worker pickup)**: `mission-workplace-knowledge-substrate` is now **🚨 P0 OPERATOR-LOCKED** with explicit pickup direction (commit `bb9df824b`, re-validated against `90fb64111`). Still TODO at audit time. Concrete: next claude-pool spin-up must direct workers at this row; don't allow normal pick-by-author to defer it. (Repeat from T21-40 P0 #1; governance state changed.)

- **P0 (new)**: Author **`mission-gateway-restart-on-merge`** (operator-decisional spec). Evidence: evaluator P0-C — live gateway 92h stale at v70 vs code v76; audit page renders 4-day-old rows. Concrete: decide policy — auto-restart on merge OR surface `schema_version` drift in conductor tick prompt. **Without this, P0 fixes #1 + #2 land but demo path still 404s.** (New.)

- **P1 (new)**: Author **`web-audit-feed-platform-events`**. Evidence: evaluator P0-D — `GET /api/v1/audit` is scoped to v67 action-record dims and never surfaces `WorkerHired` / `WorkflowHeadChanged` / `WorkflowRolledBack`. Handler DOES write to `audit_record` (verified in `routes_workers_hire.rs`); read side doesn't UNION them. ~200 LOC; high operator-trust impact. (New.)

- **P2 (persists)**: Triage the **3 AWAITING-REBASE PRs** (#1832, #1799, #1790). Unchanged across 3 audits. 1-tick read-only triage → rebase brief or close-as-superseded. (Repeat from T21-40 P2 #5; **3rd-consecutive — will escalate next audit if untouched**.)

- **P2 (new)**: Investigate **`web-pnpm-install-missing-on-cold-start`**. Evidence: evaluator side-finding — `@radix-ui/react-tooltip@1.2.8` declared in `package.json` but absent from `node_modules`; CI smoke gate didn't catch. ~50 LOC GHA workflow running `pnpm install --frozen-lockfile` on cold cache. (New.)

## 5. Resolved since last (max 5)

- 2026-05-21T21-40 🚨 ESCALATION P0 #1 (`mission-workplace-knowledge-substrate`) → **GOVERNANCE-ACTIONED** — operator promoted to `🚨 P0 OPERATOR-LOCKED` via commit `bb9df824b` after 5 consecutive auditor escalations. **Row still TODO** but now has explicit pickup direction; re-counting as P0 in §4 above pending worker claim.
- 2026-05-21T21-40 P2 #6 (3 cross-tenant follow-up CLAIMED briefs) → **RESOLVED** — all 3 MERGED: #1989 (gateway tenant lookup sweep), #1991 (prepared statements coverage audit), #1992 (tenant scope on audit detail/replay + retros). The marketing-vs-reality gap from the founder-Friday draft is now closed; "audit found bugs → fixes are MERGED" is now true.
- 2026-05-21T21-40 P1 #3 (`web-workflow-save-button-real-wire`) → **RESOLVED** — MERGED #2003 mid-interval. Phase-1 prerequisite complete; phase-2 `publish-button` (#2008) also MERGED; phase-3 `concurrent-edit-banner` CLAIMED.
- 2026-05-21T21-40 P3 #7 (`web-approvals-decider-role-on-timeline`) → **RESOLVED** — MERGED #2001 (commit `609fbea1f`). Mission claim 6 surface gap closed.
- 2026-05-21T21-40 P0 #2 (claude-pool OAuth refresh validation) → **HELD-WORKING** — claude-pool is alive this interval (workers shipped #2003, #2008, #2001, #2007 etc.); no new 401-storm in SHARED ledger since T21:39. Stale-claim drift script returns 0 rows. Not re-recommending.

## 6. Orientation score

**Swarm-orientation score: 79 / 100** (delta from last: **−2**).

Rubric (rationale, not just numbers):

- **Wave-15 buyer-trial gap closure (30%)** — strong tactical interval (21 MERGES, phase-1 wave-15 builder substrate complete via #2003 + #2008, cross-tenant follow-ups MERGED). But evaluator surfaced 2 P0 bugs that block the demo MP4 ship-gate as currently authored — 4 points withheld for that newly-visible reality. **Score: 24/30** (−3).
- **Mission-claim 1-6 substrate coverage (30%)** — claim 2 substantially advanced; claim 3 surface advanced; claim 6 closed a P3 row. Claim 4 substrate still TODO but governance moved (operator-locked). **Score: 25/30** (held).
- **Queue health (20%)** — drift script returns 0 rows; conductor + workers all alive; merge cadence ~3.5 PRs/hour through the wave. AWAITING-REBASE count unchanged (drag). **Score: 15/20** (held).
- **Drift signals (10%)** — 2 critical (eval-discovered P0 regressions) + 1 operational (binary staleness). **Score: 6/10** (−1).
- **Audit-recommendation actioning rate (10%)** — strong: 3 of 7 prior recs closed (cross-tenant follow-ups MERGED, save-button MERGED, decider-role MERGED) + 1 governance-actioned (knowledge-substrate ESCALATION → OPERATOR-LOCKED). **Score: 9/10** (+2).

The −2 net delta is purely the evaluator's P0 surfacing — the swarm shipped well, but new ground-truth made visible 2 demo-blocker bugs the conductor was not aware of. **This is exactly the value the evaluator agent was designed to add.** Orientation will rebound when P0 #1 + #2 above land.

---

**tldr:** 21 PRs MERGED in 6h (wave-15 phase-1 #2003+#2008 + 3 cross-tenant follow-ups #1989+#1991+#1992); 🚨 ESCALATION actioned (mission-workplace-knowledge-substrate now P0 OPERATOR-LOCKED); evaluator bootstrap fired and surfaced **4 P0 demo-blockers** — `;`-in-SQL-comment breaks fresh-DB migrate (regression from #1988), cookies-in-client-bundle 500s the SPA, gateway 92h stale at v70 vs code v76, audit feed misses platform events — three of those need new briefs THIS TICK; orientation **79/100 (−2)**, drop is purely evaluator visibility not regression.
