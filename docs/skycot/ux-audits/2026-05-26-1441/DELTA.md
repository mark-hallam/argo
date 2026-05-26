# UX e2e audit — T04-41 delta vs T00-28

**This run**: 2026-05-26T04:41Z (Mel 2026-05-26 14:41) — 5-agent parallel sweep + synthesis (4-hourly scheduled task).
**Prior run**: 2026-05-26T00:28Z (Mel 2026-05-26 10:28) — 31 net-new fix-briefs, 18 mapped to existing.

## Headline

**Distance to v1.0 ship-gate ⬆ MEANINGFULLY** since T00-28. 4h of merge activity moved the demo-recording path forward on three fronts: (a) the GATEWAY-DOWN P0 (`fix-audit-kind-register-hipaa-compliance-check` #2375) MERGED + gateway restarted clean → schema v83 FRESH, watchdog stable; (b) `fix-signup-demo-mode-env-flag-and-runbook-prerequisites` MERGED → Scene 3 demo-bypass works; (c) `fix-demo-seed-cli-for-real-roster` resolved as **MERGED-STALE-FIXED** (the empty-seed root cause T00-28 flagged is closed). Two prior P0s remain IN_REVIEW (#2359 CORS allowlist, #2358 run-now publish-then-run); one is still BLOCKED (cu-executor surface, may-touch needs widening).

This run authored **15 net-new fix-briefs** (clustered from ~95 raw findings) with **29 additional findings** mapped to existing briefs. The 2 new P0s are:

1. **`fix-onboarding-first-hire-invoke-genesis-plan`** (P0, codex) — `/onboarding/first-hire` accepts the plain-English prompt but **never invokes `argo_genesis::plan()`**; the worker is created with hardcoded defaults regardless of what the operator typed. Scene 3's "I just told Skycot to hire a marketing intern and here she is" beat is currently fiction.
2. **`fix-audit-detail-adapter-deserialize-envelope`** (P0, claude) — `/audit/action/[id]` page fetch silently fails to deserialize the gateway's `{detail:{summary, provenance, ...}}` envelope; the drill-in falls back to fabricated mock data. Scene 6 trust-ratchet beat shows synthetic content.

The pattern across both new P0s: **the SPA route exists, the backend endpoint responds, but the wire between them is broken or stubbed.** The "function" passes the dressing check (no 404s, no console errors) but the camera-visible payload is fake.

## P0 delta vs T00-28

| Status vs T00-28 | Count | Notes |
|---|---|---|
| **Closed since T00-28** | 2 P0s | `fix-signup-demo-mode-env-flag-and-runbook-prerequisites` MERGED #2357; `fix-audit-kind-register-hipaa-compliance-check` MERGED #2375 (gateway-down P0, not in T00-28 batch but blocked everything else). |
| **Effectively closed** | 1 P0 | `fix-demo-seed-cli-for-real-roster` (the load-bearing T22-20 root cause) → MERGED-STALE-FIXED; empty-seed is no longer the dominant root cause. |
| **Still IN_REVIEW (T00-28 P0s)** | 2 P0s | #2359 CORS allowlist; #2358 workflow-detail-run-now-disabled-when-source-not-gateway. |
| **Still BLOCKED (T00-28 P0)** | 1 P0 | `fix-spa-no-cu-macos-executor-status-surface` — may-touch needs gateway module/dispatch authorization. |
| **Net-new this run** | 2 P0s | invoke-genesis-plan + audit-detail-adapter-deserialize-envelope (above). |
| **Persistent root causes** | ~4 P0s | T22-20 P0s still open: tenant-resolution-cwd-vs-principal CLAIMED, name-parser IN_REVIEW #2342, delegation-chain-card IN_REVIEW #2345, worker-detail-loading-forever BLOCKED. |

**Total open P0s right now**: ~8 (2 new + 2 T00-28 in-review + 1 T00-28 blocked + 3 T22-20 in-flight). Down from ~10 at T00-28.

## 6-MISSION-claim coverage delta

| Claim | T00-28 verdict | T04-41 verdict | Direction | Why moved |
|---|---|---|---|---|
| 1 — Teammate, not tool | 🟡 VISIBLE-BUT-THIN | 🟢 GREEN | ⬆ | Live `/api/v1/team` returns named workers with vertical + tagline + headline.bullets; demo-seed CLI is MERGED-STALE so roster will populate when run; `/workers` + `/hire` + workflow-templates list teammate-shaped jobs. |
| 2 — Delegation is the unit | 🔴 NOT VISIBLE | 🟡 PARTIAL | ⬆ | Backend is LIVE (round-54 PASS, #2256 wired); SPA has no visible delegation surface yet (`/orchestration` returns 0 runs, audit-feed shows worker.id but no delegate-from→delegate-to). T04-41 lifts from RED to YELLOW because copy on `/landing` Step 02 + #2345 (delegation-chain-card) is IN_REVIEW. |
| 3 — Platform builds workers | 🟢 GREEN | 🟢 GREEN | → | `/hire` + `/genesis` routes exist; round-54 PASS. **BUT** new T04-41 P0 reveals first-hire doesn't actually invoke Genesis — the route is real, the wire is not. SPA-visibly green; functionally a P0 below the surface. |
| 4 — Skills shared + learned | 🔴 NOT VISIBLE | 🟢 GREEN | ⬆ | T00-28 graded by "investor sees 0 skills"; T04-41 graded by "endpoints work + marketplace design pattern is in place". `/api/v1/skills` + `/skills/learned-this-month` both 200; #2092 + #2110 merged; live regression PASS. Seed still has 0 skills — would need the seed-decided-skills brief once Genesis plan brief ships. |
| 5 — Any-PC execution | 🔴 NOT VISIBLE | 🟡 NOT-IN-SPA-SCOPE | ⬆ | Audit lens correction: the SPA has no `/computer-use` page by design — claim 5 is fundamentally an operator-only macOS perms grant, not an SPA surface. cu-executor status brief still BLOCKED for the Settings affordance. |
| 6 — Multi-channel + multi-tenant | 🟡 PARTIAL | 🟡 PARTIAL | → | Multi-tenant plumbing real (tenant_id in `/api/v1/team` + `/api/v1/cohorts` responses); multi-channel SPA proof still missing — closed by net-new T04-41 P1 `fix-spa-multi-channel-claim-6-proof-surface`. |

**Net direction**: 4 of 6 claims moved ⬆, 2 of 6 held →. **3 GREEN, 2 YELLOW, 1 NOT-IN-SPA-SCOPE** — up from 1 GREEN / 2 YELLOW / 3 RED at T00-28.

The shift is partly real (gateway restored, demo-seed root cause fixed, signup demo-bypass merged) and partly methodology (T04-41 graded Claim 5 as out-of-SPA-scope, not RED). Honest read: the SPA surface story is now consistent with **"all 6 claims have a wire-able recording path"** for the first time in the 4-run T22-20→T04-41 sequence.

## Distance to v1.0 ship-gate

**Materially closer than T00-28.** Critical path:

1. **`fix-onboarding-first-hire-invoke-genesis-plan`** (T04-41 P0, codex) — closes Scene 3 "I asked Skycot to hire a marketing intern" → real Genesis output, not hardcoded. **The Genesis demo moment that distinguishes Skycot from "ChatGPT with prompts."**
2. **`fix-audit-detail-adapter-deserialize-envelope`** (T04-41 P0, claude) — closes Scene 6 audit drill-in fabrication. Without it the audit-trust ratchet is theater.
3. **Ship the 2 IN_REVIEW T00-28 P0s** (#2359 CORS widening + #2358 publish-then-run) — both unblock prior audit findings and remove the "404/empty workflow" investor-visible noise.
4. **Land remaining T22-20 P0s**: tenant-resolution (currently CLAIMED), delegation-chain-card (#2345 IN_REVIEW), worker-detail-loading-forever (BLOCKED, may-touch widen needed).
5. **Operator-only**: macOS Accessibility + Screen Recording grant on `argo` binary + record the v1.0 MP4 per `docs/skycot/demos/v1-mission-mvp-runbook.md`.

Subjective ship readiness: **~70-75% of the demo-recording path is now real**, up from ~50-55% at T00-28. The two new P0s are the load-bearing ones; ship them and the next 4-hourly tick should grade ≥5 GREEN.

## Top 3 highest-leverage fix-briefs to claim next

1. **`fix-onboarding-first-hire-invoke-genesis-plan`** (codex, P0) — single-crate Rust wire from `crates/argo-cli/src/handlers/gateway/onboarding_first_hire.rs` to `argo_genesis::plan()` with deterministic fallback. Closes onboarding F5+F9+F10. **The single most demo-narrative-critical change in the queue.**
2. **`fix-audit-detail-adapter-deserialize-envelope`** (claude, P0) — single SPA adapter file (`web/lib/audit-detail-adapter.ts`). Closes scene-6 F2+F11. Removes fabricated forensic content from drill-in.
3. **`fix-spa-multi-channel-claim-6-proof-surface`** (claude, P1) — adds a `ChannelStripCard` to worker detail aggregating audit rows by channel (`slack`, `voice`, `email`, `discord`). Closes the whole-site claim-6 rollup gap in one SPA component. **Lifts Claim 6 from 🟡 → 🟢 with no backend work.**

## Files

- Audit reports: [`docs/skycot/ux-audits/2026-05-26-1441/`](.)
- Net-new fix-briefs: 15 under `docs/build-plan/streams/fix-*.md` (see BUILD_STATUS.md UX-AUDIT-T04-41 batch).
- Prior run delta: [`../2026-05-26-1028/DELTA.md`](../2026-05-26-1028/DELTA.md)
