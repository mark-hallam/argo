# UX e2e audit — T00-28 delta vs T22-20

**This run**: 2026-05-26T00:28Z (Mel 2026-05-26 10:28) — 5-agent parallel sweep + synthesis (4-hourly scheduled task).
**Prior run**: 2026-05-25T22:20Z (Mel 2026-05-26 08:20) — 18 net-new fix-briefs authored, 17 mapped to existing.

## Headline

**The SPA story REGRESSED vs the T22-20 narrative** — but the regression is mostly a methodology correction, not new breakage. T22-20 scored 4 of 6 MISSION claims 🟢 GREEN with a code-side-LIVE counted-as-GREEN lens; T00-28's whole-site agent applied a strict **"what does an investor watching the SPA actually see"** lens and scored only **1 of 6 GREEN, 2 🟡 YELLOW, 3 🔴 RED**. The load-bearing root cause: **the gateway seed is empty** (1 worker `Maya/marketing-intern`, 0 workflows, 0 approvals, 0 skills, 0 agent-turn audit rows). Without `fix-demo-seed-cli-for-real-roster` (already on disk, claimed-not-shipped), the SPA cannot demonstrate any claim regardless of how complete the code is.

**31 net-new fix-briefs** authored this pass (clustered from ~75 raw findings). **18 additional findings** mapped to existing fix-briefs already on disk (no duplicates written). The 4 new P0s are:

1. **fix-gateway-cors-localhost-127-and-preview-proxy-origins** (P0, codex) — gateway CORS allowlist is single-origin literal `localhost:3000`; any 127.0.0.1 form or Claude-Preview proxy origin silently fails on every SPA fetch.
2. **fix-signup-demo-mode-env-flag-and-runbook-prerequisites** (P0, claude) — `/auth/signup` has no demo bypass; recording flow aborts in Scene 3 without manual SMTP + env-var setup.
3. **fix-workflow-detail-run-now-disabled-when-source-not-gateway** (P0, claude) — Run-now button is permanently disabled when gateway has no published workflows; Scene 5 "Hit Run" beat is unreachable from the seed.
4. **fix-spa-no-cu-macos-executor-status-surface** (P0, claude) — SPA has zero affordance confirming computer-use-macos is connected; Claim 5 cannot be self-verified before recording.

## P0 delta vs T22-20

| Status vs T22-20 | Count | Notes |
|---|---|---|
| **Closed since T22-20** | 0 P0s | All 6 T22-20 P0 fix-briefs are still in flight: 4 IN_REVIEW (#2342 / #2343 / #2345 / #2347), 1 CLAIMED (`fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`), 1 BLOCKED (`fix-worker-detail-loading-forever-no-error-boundary`). No P0 PR has landed in the 2h since. |
| **Net-new this run** | 4 P0s | CORS allowlist, signup demo-bypass, run-now-disabled, no-cu-executor-surface (above) |
| **Persistent root causes (still open across runs)** | ~6 P0s | demo-seed CLI not run, persona avatars unbundled, schedules-tab end-to-end, customer_research_followup workflow, competitor-pricing skill seed, cross-tenant cap-bypass behaviour |

The **gateway-CORS finding** is the biggest blast-radius surprise of this run — every non-localhost dev session (including the Claude-Preview proxy this audit ran against) silently fails on SPA→gateway calls. Combined with the empty-seed observation, the "SPA shows nothing" pattern across reports could be a CORS issue masking a perfectly-good gateway, **not** an empty seed. The codex brief widens the allowlist; once shipped, the next audit should distinguish the two.

## 6-MISSION-claim coverage delta

| Claim | T22-20 verdict | T00-28 verdict | Direction | Why moved |
|---|---|---|---|---|
| 1 — Teammate, not tool | 🟢 GREEN | 🟡 VISIBLE-BUT-THIN | ⬇ | Seed has only `Maya` (marketing-intern) with `headline.bullets: ["No activity yet."]`; teammate metaphor cannot demonstrate without the 5-person roster |
| 2 — Delegation is the unit | 🟢 GREEN | 🔴 NOT VISIBLE | ⬇ | T22-20 P0 `fix-delegation-chain-card-visual-surface` is IN_REVIEW #2345 — still no SPA surface visible; the brief lands next run if PR merges |
| 3 — Platform builds workers | 🟡 YELLOW | 🟢 VISIBLE | ⬆ | `/settings/providers`, `/settings/connectors`, `/onboarding/connect-provider`, `/cohorts/new`, hire wizard + Genesis Phase-1 all 200; full UI surface for build-workers story |
| 4 — Skills shared + learned | 🟢 GREEN | 🔴 NOT VISIBLE | ⬇ | Skill marketplace returns 0 skills; `/skills/learned-this-month` 404s; T22-20 had not noticed seed-zero — actually a methodology correction, not a regression |
| 5 — Any-PC execution (macOS cu) | 🟡 YELLOW | 🔴 NOT VISIBLE | ⬇ | No SPA surface confirms cu-macos is connected; closed by net-new P0 `fix-spa-no-cu-macos-executor-status-surface` this run |
| 6 — Multi-channel + multi-tenant | 🟢 GREEN | 🟡 PARTIAL | ⬇ | Audit feed has only operator-browsing rows (`worker.name: unknown`), no agent-turn rows or cross-tenant denial events |

**Net direction**: 1 of 6 claims moved ⬆, 5 of 6 moved ⬇. **1 of 6 GREEN, 2 of 6 YELLOW, 3 of 6 RED.**

**This is not a real regression** — the code claims that flipped 🟢→🔴 (Claim 4 in particular) are still 🟢 in CLAUDE.md's strict "live regression test passes cold" sense; what changed is the audit lens. T22-20 graded the SPA on what the page wireframes *could* show with seed data; T00-28 graded on what an investor sees *right now* opening the dev server cold. The honest read: **without `fix-demo-seed-cli-for-real-roster` shipped + executed, the SPA cannot demonstrate any of the 6 claims, regardless of code state.**

## Distance to v1.0 ship-gate

**Approximately the same as T22-20** — no P0 PR landed in the 2h between runs, so the queue depth hasn't shifted, but the empty-seed problem is now sharply diagnosed. Critical path:

1. **Demo seed CLI** (`fix-demo-seed-cli-for-real-roster`, on disk, currently CLAIMED) — single highest-leverage item; closes ~10 findings across 4 of 5 audit reports.
2. **Land the 6 T22-20 P0 PRs in review** — #2342 / #2343 / #2345 / #2347 + the 2 still-CLAIMED / BLOCKED ones.
3. **Gateway CORS widening** (net-new P0 this run) — unblocks any dev session NOT on `localhost:3000`.
4. **Signup demo-bypass + run-now publish-then-run + cu-executor surface** (3 other net-new P0s) — direct demo-recording prerequisites.
5. **Operator-only**: macOS perms grant + record the MP4.

Order: ship the seed CLI first; rerun T04-28 (next 4-hourly tick) against a seeded gateway and the rollup should jump back to ≥4 GREEN.

## Top 3 highest-leverage fix-briefs to claim next

1. `fix-gateway-cors-localhost-127-and-preview-proxy-origins` (codex, P0) — single change in `crates/argo-cli/src/handlers/gateway/`; un-silences the entire SPA→gateway path on non-canonical origins. Could turn out to retroactively close several "SPA shows nothing" findings across the prior 2 runs.
2. `fix-demo-seed-cli-for-real-roster` (on disk, CLAIMED, not net-new) — the load-bearing seed problem; 5 personas + workflows + skills + audit rows in one CLI invocation. Without this, no audit lens reads ≥4 GREEN.
3. `fix-spa-no-cu-macos-executor-status-surface` (claude, P0) — closes Claim 5 from "operator-only black box" to "SPA-visible affordance" with a single Settings tab; required for the recording's Scene 5.
