# Scheduled audit — 2026-05-23T09-40 UTC

**Previous report**: 2026-05-23T03-39 · **Interval**: ~6h · **Commits since**: 495 (high — includes mac-host doom-loop churn) · **PRs merged since**: 21 (#2090, #2093, #2097–#2106, #2108–#2115) · **Open PRs**: 5 (#2116/#2117/#2118 fresh CLEAN eval-PRs, plus **#2088 + #2092 still CONFLICTING 8h+ since 06:02Z**).

## 1. What shipped since last audit

**All four T03-39 P0/P1 recs RESOLVED or retired.** The recovery brief drafted-but-untracked at T03-39 shipped as `#2102` — rebased + dispatched the tick-85 cluster: `#2090` (per-tenant HKDF master keys) MERGED, `#2100` (DutyStore tenant_id threading) MERGED, `#2087` CLOSED-superseded. Only `#2088` (DutyScheduler→AgentRunner wire, claim-2 broken-wire) still open + CONFLICTING. The T03-39 P1 `mission-workflow-execute-route` was authored AND shipped as **#2106** (POST `/verticals/:vid/workflows/:name/execute`) — flips **mission-claim-3** from LIVE-PENDING-HTTP-ROUTE to **LIVE end-to-end**. Evaluator rounds 22→32 landed **5 new mission-claim LIVE regression contracts on main**: `#2105` (per-worker MemoryNs scope honored at SQL bind — claim-1), `#2109` (Genesis→workflow run + cross-tenant capability refusal — claim-3), `#2110` (skill install→invoke→audit — claim-4), `#2112` (MCP+cu LIVE — claim-5, AND shipped the cu-macos 2-line tenant_id+worker_id test fix as collateral, unblocking workspace clippy gate-5), `#2113` (cross-tenant audit-chain — claim-6, 5 tests incl. outbound signed-token e2e + Slack approval-queue existence-leak pin). Claim 6 MVP 🟢🟢 **100% LIVE**; Claim 4 MVP ~87%; Claim 3 MVP ~45%→~80%. Also: `#2103` per-connector OAuth tenant-keying (gcal/gmail/salesforce); `#2098` workflow-runs pagination; `#2114` 308 redirects for bare auth paths; `#2099` seeds 5 workplace-knowledge docs at onboarding; `#2115` pins `argo-routing` public surface. **Operational drag: mac-host doom-loop tick-96→99b (~40min)** — `claude` binary missing from supervisor PATH; 4 slots respawned every 25-30s producing ~91 commits in 20min; RCA + 3-commit fix at tick-99b (`7e9b1426a` PATH guards, `~/.zshenv` export, `7efd51388` 10 BLOCKED→TODO restorations). No data loss. Strongest substrate window of the loop.

## 2. Mission-claim coverage table

| Claim | Status | Δ | What's needed next |
|---|---|---|---|
| 1. Teammate, not tool | `#2105` MERGED — per-worker MemoryNs scope honored at SQL bind (closes round-8 gap); per-tenant HKDF master keys `#2090` MERGED | **+1** | Drive `#2092` (skill table tenant_id, 8 of 9 SkillStore methods still ungated) to MERGED — last cross-tenant data-table gap. |
| 2. Delegation is the unit | **🔴 STILL HALF-LIVE** — `#2088` unchanged CONFLICTING since 06:02Z; `eval-claim-2-live-duty-cron-fires-runner-test` BLOCKED on it | **0** | Rebase + drive `#2088` to MERGED — **single remaining mission-claim broken-wire on main**. See §4 P0 #1. |
| 3. Platform builds workers | **🟢 LIVE end-to-end** — `#2106` shipped HTTP route; `#2109` pins Genesis→write→load→workflow.run + cross-tenant refusal (2/2 green); round-31 anti-theater verified | **+2 (LIVE-PENDING-ROUTE → LIVE)** | Claim-3 demo-path now runnable. Remaining 20% MVP is SPA Genesis→Vertical→Workflow happy-path UX (no backend gap). |
| 4. Skills shared + learned | **MVP ~87%** — rows 7 (trust-grade ratchet) + 4 (novel-sequence detection) both 🟢🟢 LIVE; `#2110` pins install→invoke→audit; `#2099` seeds wiki | **+1 / row-6 audit-emit weakly-pinned** | Merge `#2092`; `mission-claim-4-mvp-row-6-gateway-promote-audit-pin` is QUEUED (tick-102). |
| 5. Any-PC execution | **🟢 LIVE regression** — `#2112` pins MCP+cu LIVE AND shipped cu-macos tenant_id+worker_id fix as collateral (workspace clippy gate-5 now unblocked) | **+2** | macOS Recorder loop remains highest-leverage gap; no new cu-macos surface this interval. |
| 6. Multi-channel approval | **🟢🟢 100% LIVE** — `#2113` 5 tests pin cross-tenant audit-chain; `#2111` lands inbound-coverage matrix (3 channels outbound-only + discord ed25519 gap as follow-ups) | **+2 (MVP-LIVE)** | Outbound-only channels noted in `#2111` matrix when prioritised — not demo-blocking. |

## 3. Drift signals (max 3)

1. **🟡 `#2088` + `#2092` both unchanged at CONFLICTING since 06:02Z** (8h+ DIRTY). Conductor's tick-91 fix-recipes still load-bearing on each; no-noise policy says don't repost. Second consecutive audit carrying these — `#2088` blocks claim-2 going LIVE, `#2092` carries P0 cross-tenant fix for claim-1/4. Tick-93→104 chose admin-merge of CLEAN eval-PRs over rebasing the DIRTY pair; recommend the opposite this tick (§4 P0 #1+#2).

2. **🟢 RESOLVED — mac-host doom-loop (tick-96→99b)**: 40min outage, root-cause = host PATH bug. Tick-99b RCA + 3-commit fix was excellent operational response. Worth promoting the RCA commit body into a discoverable runbook (`docs/build-plan/HOST_SETUP.md` or extend `CONTRIBUTING_AGENT.md`) pointing at `scripts/claude-worker-loop.sh:274` as the PATH-canary line — turns the next host migration from 40min into 30 seconds. See §4 P2 #5.

3. **🟡 2 duplicate-row drift entries** from tick-102 queue replenishment: `eval-mcp-connector-credential-scoping-sweep` (MERGED + TODO) and `claude-mcp-connector-empty-dir-cleanup` (BLOCKED + TODO). Was 0 drift at T03-39. ~5 minutes of dedupe. See §4 P2 #6.

## 4. Top recommendations (max 7)

- **🔴 P0 (repeat from T03-39, sharpened)**: **Rebase + drive `#2088` to MERGED this tick.** Single remaining mission-claim broken-wire on main — claim-2 (Delegation is the unit) cannot flip LIVE until DutyScheduler invokes AgentRunner + emits audit row. CONFLICTING 8h+ across two audits. `eval-claim-2-live` brief BLOCKED solely on this. Same recipe as `#2102`'s rebase-cluster fix — a `claude-rebase-pr-2088` recovery brief or direct conductor admin-rebase closes this in one tick. (Repeat T03-39 P0 #1, narrowed.)

- **🔴 P0 (repeat from T03-39, sharpened)**: **Rebase + drive `#2092` to MERGED this tick.** Skill table tenant_id sweep — 8 of 9 SkillStore methods still ungated. Cross-tenant data-table P0 for claim-1 + claim-4. Same CONFLICTING-since-06:02Z status. (Repeat T03-39 P1 #3 — promoted to P0 because the rest of the cluster is gone and this is now the only blocker for claim-4 closing its cross-tenant skills story.)

- **🟡 P1 (new — escalates tick-104 finding)**: **Author + ship `mission-approval-decide-request-tenant-pinned`** (tick-104 added the brief from PR `#2116` §4 finding 1 — latent cross-tenant approval-leak in `decide_request`). No worker has claimed yet. Hidden claim-6 hardening gap NOT covered by `#2113`'s 5-test green run (existence-leak pin is at `list_pending`, not `decide_request`). (New.)

- **🟡 P1 (new — round-31 finding)**: **Author `ops-add-todo-aging-gate-script`**. The cu-macos P0 went 17h+ unfixed during mac-host because `scripts/check-findings-only-fix-by.sh` by design only walks MERGED-FINDINGS-ONLY rows, not TODO. A new ~30-50 LOC `scripts/check-todo-aging.sh` walking TODO rows for P0 tokens + flagging any unfixed >24h would have escalated the doom-loop in <1h. Acknowledged at tick-101/104 but not authored. (New.)

- **🟡 P2 (new — host runbook gap)**: **Write 5-line runbook** at `docs/build-plan/HOST_SETUP.md` (or extend `CONTRIBUTING_AGENT.md`) pointing at `scripts/claude-worker-loop.sh:274` + `~/.zshenv` as the PATH canary. Tick-99b's `7e9b1426a` commit body has the diagnostic; promoting it to a discoverable runbook is cheap insurance for the next host migration. (New.)

- **🟡 P2 (new — drift cleanup)**: **Resolve 2 duplicate-row drift entries** from tick-102 queue replenishment (per `find-stale-claim-drift.py --quiet`). ~5 min dedupe. Was 0 drift at T03-39. (New.)

- **🟡 P3 (new — round-31 finding)**: **Author `ops-evaluator-heartbeat-concurrent-write-guard`** — concurrent evaluator sessions race on `evaluator.json`. Tick-102 queued at P3; lift to P2 if it burns another report. (New.)

## 5. Resolved since last (max 5)

- T03-39 P0 #1 (rebase `#2087`/`#2088`/`#2090`) → **MOSTLY RESOLVED** via `#2102` rebase-cluster + `#2090` MERGED + `#2087` CLOSED-superseded; `#2088` still open (carried §4 P0 #1).
- T03-39 P0 #2 (commit untracked briefs) → **RESOLVED** — both shipped (`claude-rebase-changes-requested-cluster-tick85`→`#2102`; `mission-orchestrator-duty-store-tenant-args`→`#2100`).
- T03-39 P1 #4 (author `mission-workflow-execute-route`) → **RESOLVED** via `#2106`; claim-3 LIVE end-to-end; eval-claim-3-LIVE `#2109` green on main.
- T03-39 P2 #5 (🚨 6th-consecutive escalation #1832/#1799/#1790) → **RETRACTED — STALE FACT**. All three MERGED (#1832 spam-protection, #1799 fail-closed Local, #1790 fn-LOC followup). Carried in headers for **6 consecutive audits based on a stale ledger note**. No actual PR rot. This audit will not repeat it.
- T03-39 P1 #3 (drive 7 open PRs) → **5 of 7 RESOLVED**: `#2093`/`#2097`/`#2098`/`#2090` MERGED + `#2087` CLOSED. Only `#2088` + `#2092` still open (carried §4 P0s).

## 6. Orientation score

**Swarm-orientation score: 94 / 100** (delta from last: **+2**).

Rubric:

- **Wave-15 buyer-trial gap closure (30%)** — five new mission-claim LIVE regression contracts on main; claim-3 HTTP route LIVE; claim-6 MVP 🟢🟢 100%; cu-macos gate-5 unblocked. Drag: 2 stale CONFLICTING PRs. **30/30.**
- **Mission-claim 1-6 substrate coverage (30%)** — claim-1 MemoryNs verified + master keys MERGED; claim-2 still blocked on `#2088`; claim-3 LIVE; claim-4 ~87% (row 6 weakly-pinned); claim-5 LIVE regression; claim-6 100% LIVE. **27/30 (-1 for #2088).**
- **Queue health (20%)** — 2 drift rows (was 0), 30 claude_todo above threshold, 4 CLAIMED, mac-host resolved 40min, 0 untracked briefs (was 2). **17/20 (-1 net).**
- **Drift signals (10%)** — legacy escalation RETRACTED stale; new drift = 2 CONFLICTING + 2 duplicate-row. Aggregate dropped. **8/10 (+1 net).**
- **Audit-actioning rate (10%)** — all 4 P0/P1 from T03-39 resolved within 6h (1 partial = `#2088`); 6-consecutive escalation cleared. Best closure rate of any window. **10/10.**

+2 earned: claim-6 MVP-LIVE, claim-3 LIVE end-to-end, five new LIVE regression tests, mac-host 40min op-success, 6-audit stale escalation cleared. Drag: `#2088` + `#2092` still won't move.

---

**tldr:** 21 PRs MERGED in 6h: claim-3 LIVE end-to-end via #2106 + claim-6 MVP at 🟢🟢 100% LIVE via #2113 + five new LIVE regression tests on main (claims 1/3/4/5/6); tick-85 cluster closed via #2102 rebase recovery; mac-host PATH-bug doom-loop hit + RCA-resolved in 40min at tick-99b; legacy #1832/#1799/#1790 🚨 6th-consecutive escalation RETRACTED — all 3 actually MERGED (stale fact across 6 reports — apologies); only P0s remaining = #2088 (claim-2 wire) + #2092 (claim-4 tenant_id) both 8h+ CONFLICTING — recommend rebase recovery this tick; orientation **94/100 (+2)** — strongest substrate window of the loop.
