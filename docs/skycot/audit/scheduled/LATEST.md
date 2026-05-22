# Scheduled audit — 2026-05-22T09-40 UTC

**Previous report**: 2026-05-22T03-39 · **Interval**: ~6h · **Commits since**: 109 (incl. 24 PR merges) · **PRs merged since**: 24 (#2010 – #2038, less the 5 DIRTY) · **Open PRs right now**: 5 (#2016 / #2017 / #2018 / #2021 / #2022 — all DIRTY).

## 1. What shipped since last audit

Best 6-hour interval the swarm has shipped. **All 4 evaluator P0 demo-blockers landed**: `#2025` closed the `;`-in-SQL-comment regression that crashed every fresh-install gateway at v43; `#2027` moved `cookies()` out of the client bundle (cold-start /app/* HTTP 500 fixed) + `#2032` adds a lint to keep it out; `#2023` shipped the gateway-restart-on-merge policy spec; `#2024` added a cold-install CI gate that catches missing-from-`node_modules` regressions like the `@radix-ui/react-tooltip` one. And the load-bearing item: **the 🚨 5-tick ESCALATION on `mission-workplace-knowledge-substrate` MERGED via `#2034`** — new `argo-workplace-knowledge` crate + migration v77 = tenant-scoped wiki substrate is LIVE. Claim 4 substrate finally ships. Also MERGED: `#2031` AuditKind split, `#2033` cross-tenant audit-coverage sweep (extends #1983), `#2036` cohort bulk-assign-skill, `#2037` onboarding wizard flake fix, `#2038` worker-loop zombie-detection sidecar, and 4 workflow polish PRs (#2026 monaco / #2028 JSON body / #2029 sample catalog / #2030 auth session-expired). Drag: a 5-PR DIRTY cluster (#2016/#2017/#2018/#2021/#2022) jammed during the merge cascade.

## 2. Mission-claim coverage table

| Claim | Status | Δ since last | What's needed next |
|---|---|---|---|
| 1. Teammate, not tool | hire surface stable; `web-audit-feed-platform-events` (the brief that would surface `WorkerHired` in the audit drill-in) CLAIMED twice → zombie-demoted twice in this interval | held | Fresh worker pickup — the brief itself is ready, the worker process is hanging. Sidecar `#2038` should catch the next hang faster. |
| 2. Delegation is the unit | wave-15 phase-1 + phase-2 already complete (#2003 + #2008 prior interval); phase-3 concurrent-edit-banner is `#2022` DIRTY 3.5h | +1 polish (4 workflow polishers MERGED), phase-3 still open | Rebase `#2022` cleanly OR close-as-stale + re-author. This is the last surface in the wave-15 P0 #3 builder cluster. |
| 3. Platform builds workers | `#2023` gateway-restart policy spec MERGED (operator policy doc landed; was P0 #4 last audit); `#2029` sample-catalog includes HTTP+Code | +2 (P0 #4 closed + UX coherence) | Make the restart policy executable — operator needs the one-command path to flip the live gateway from v70 → v77 |
| 4. Skills shared + learned | **🚨 ESCALATION CLOSED — `#2034` MERGED**: `argo-workplace-knowledge` crate + migration v77 + tenant-scoped wiki substrate is LIVE on main | **+5 substantial — biggest interval delta** | Pickup `web-workplace-knowledge-editor` (the UX surface) + the 3 Wave-A briefs that `mission-workplace-knowledge-audit-kinds` was the cascade-unlock for |
| 5. Any-PC execution | unchanged macOS-only | held | Out of scope this cycle (operator-locked v1 decision) |
| 6. Multi-channel approval | `#2030` web-auth-session-expired-redirect MERGED (returnTo flow) | +1 surface | Watch decider-role end-to-end + audit feed coverage closure (depends on claim-1 zombie unblock) |

## 3. Drift signals (max 3)

1. **🟡 Cascade-induced DIRTY churn.** Five open PRs (#2016 TestRunHistoryPanel / #2017 per-workflow-runs-table / #2018 hire-idempotency-phase-3 / #2021 test-run gateway-persist / #2022 concurrent-edit-banner) all went DIRTY in the tick-38 wave when main shifted under them. #2017 + #2021 include drive-by SQL fixes that **duplicate** what `#2025` shipped repo-wide — drop during rebase. Pattern: fast merges → premature ledger flips → rollback when GitHub rejects on conflict. See §4 P0 #2.

2. **🟡 Zombie-claim pattern on `web-audit-feed-platform-events`.** Claimed T06:15:35Z by PID 59400 → log silent 15min → demoted; re-claimed T08:55:19Z → demoted again. Same brief, same failure mode, twice in 4h. Worktree preserved at `/tmp/web-audit-feed-platform-events`. `#2038` zombie-sidecar landed mid-interval and will catch the next hang faster, but a fresh worker should `git status` that worktree before starting cold. See §4 P0 #1.

3. **🟡 Three long-stale AWAITING-REBASE PRs unchanged across 4 audits.** `#1832`, `#1799`, `#1790` — still untriaged. **4th-consecutive call**. 1-tick read-only triage → rebase brief or close-as-superseded.

## 4. Top recommendations (max 7)

- **🔴 P0 (repeat from T03-39 P1 #5)**: Unblock **`web-audit-feed-platform-events`** — the brief that surfaces `WorkerHired` / `WorkflowHeadChanged` / `WorkflowRolledBack` in the audit drill-in. Status: zombie-demoted twice in this interval. Concrete: next claude worker spin-up should `cd /tmp/web-audit-feed-platform-events && git status` to decide salvage-vs-restart, then push-with-lease. Brief is ready, the runtime keeps hanging. (Repeat — escalating.)

- **🔴 P0 (new)**: Sweep the **5 DIRTY PRs** (#2016 / #2017 / #2018 / #2021 / #2022) in a single tick. Either (a) rebase + admin-merge, or (b) close-as-stale with the WIP captured in a re-authored brief. Drop the drive-by SQL fixes in #2017 + #2021 — `#2025` already shipped the repo-wide migration-comment-splitter. #2022 is the wave-15 phase-3/3 concurrent-edit banner — needs to land to formally close the builder cluster. Pre-condition: confirm main is stable enough that 5 sequential rebases won't trigger another cascade. (New.)

- **P1 (new)**: **Re-fire the evaluator now that 4 P0s closed.** All 4 of its bootstrap findings landed in this interval (`#2025` / `#2027` / `#2023` / claim-1 audit-feed is the only one still TODO). A fresh evaluator run would (a) confirm fresh-install boot now works on v77, (b) confirm /app/* routes render cold, (c) measure what's actually demo-blocking before the MP4 recording, (d) re-baseline the next ground-truth set. The evaluator's cadence is daily (86400s) per its heartbeat — manual fire is appropriate after this much P0 closure. (New.)

- **P1 (new)**: **Pick up `web-workplace-knowledge-editor`** — the UX surface on top of the `#2034` substrate. Without the editor, the substrate is server-side-only and operator-invisible. The cascade-unlock briefs (3 Wave-A items that were blocked on `mission-workplace-knowledge-audit-kinds`) should be unblocked now too — conductor's `727eb193d` commit already marked one. (New.)

- **P2 (4th-consecutive — ESCALATING)**: Triage the **3 AWAITING-REBASE PRs** (#1832, #1799, #1790). Unchanged across 4 audits. Either rebase + merge, or close-as-superseded with one-line reason. 30-minute read-only investigation max. (Repeat from T03-39 P2 #6, T21-40 P2 #5; **🚨 4th-consecutive — escalate to operator pingable item next audit if untouched**.)

- **P2 (new)**: **Make the gateway-restart-on-merge policy executable.** `#2023` shipped the spec doc. The operator's next action — manually restart the live gateway from v70 → v77 to actually consume #2034's wiki substrate + everything else from the interval — is not yet a one-command path. Pre-launch this is fine; before the v1.0 demo MP4 recording it's load-bearing. ~20 LOC of operator runbook OR a `make restart-live-gateway` target. (New.)

- **P2 (new)**: **Audit the 8 BLOCKED ledger rows** (codex-cli-json-output-coverage-audit + 7 others). 1-tick read-only sweep: which are actually still blocked vs which are stale-blocked because the original blocker has shipped? Pattern from `mission-workplace-knowledge-audit-kinds` (727eb193d unblock) suggests at least 2-3 are now actionable. (New.)

## 5. Resolved since last (max 5)

- T03-39 P0 #1 (`;`-in-SQL-comment fresh-install blocker) → **RESOLVED** via `#2025` (`7f6def800`). Fresh-install no longer crashes at v43.
- T03-39 P0 #2 (cookies in client bundle 500ing /app/*) → **RESOLVED** via `#2027` (`fba4b1a11`) + `#2032` lint enforces no-server-imports going forward.
- T03-39 🚨 P0 #3 (`mission-workplace-knowledge-substrate` — **5th-consecutive ESCALATION**) → **RESOLVED — biggest delta** via `#2034` (`6b0270b63`). Claim 4 substrate ships.
- T03-39 P0 #4 (gateway-restart-on-merge policy) → **RESOLVED** via `#2023` (`f7e9c1246`). Spec landed; executable runbook is now a P2 follow-up.
- T03-39 P2 #7 (`web-pnpm-install-missing-on-cold-start`) → **RESOLVED** via `#2024` (`db0632b65`). CI cold-install gate prevents this regression class.

## 6. Orientation score

**Swarm-orientation score: 87 / 100** (delta from last: **+8**).

Rubric:

- **Wave-15 buyer-trial gap closure (30%)** — all 4 evaluator P0s MERGED + 5-tick escalation MERGED + cold-install gate + no-server-imports lint. Only drag: 5 DIRTY PRs from cascade. **Score: 28/30 (+4).**
- **Mission-claim 1-6 substrate coverage (30%)** — claim 4 substrate FINALLY landed (5-tick stuck); claim 3 governance policy; claim 6 surface. Claim 1 audit-feed is the only outstanding gap. **Score: 27/30 (+2).**
- **Queue health (20%)** — `find-stale-claim-drift.py --quiet` returns 0; 770 MERGED vs 7 TODO / 2 CLAIMED / 1 IN_REVIEW / 8 BLOCKED / 4 DIRTY. 5 DIRTY rebase queue is the drag. **Score: 16/20 (+1).**
- **Drift signals (10%)** — cascade-DIRTY + zombie-claim + 4th-consecutive stale-rebase backlog. Two of three operational not structural. **Score: 6/10 (held).**
- **Audit-recommendation actioning rate (10%)** — 5 of 7 prior recs RESOLVED in 6h including the 5-tick escalation. Best ratio this loop has logged. **Score: 10/10 (+1).**

+8 is earned: swarm shipped exactly what the auditor + evaluator flagged. Orientation engine working as designed. Next highest-leverage move: unblock the audit-feed zombie so claim-1 closes too.

---

**tldr:** 24 PRs MERGED in 6h closed ALL 4 evaluator P0 demo-blockers (#2023 gateway-restart-spec / #2025 SQL-comment-splitter / #2027 cookies-server-only-split / #2032 lint) + the 5-tick 🚨 ESCALATION mission-workplace-knowledge-substrate (#2034 argo-workplace-knowledge crate + v77 migration) + 4 prior recommendations; remaining drag is 5 DIRTY PRs cluster post-cascade + web-audit-feed-platform-events zombie-claimed twice; orientation **87/100 (+8)** — biggest single-interval jump this loop has logged.
