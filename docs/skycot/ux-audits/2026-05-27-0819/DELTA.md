# UX e2e audit — T0819MEL delta vs prior (T0739MEL, 40 min ago)

**Window**: 2026-05-27T07:39 → T08:19 Melbourne (2026-05-26T21:39 → T22:19Z UTC).

## Headline

**3 P0s materially closed (backend); 4 net-new P0s surfaced (SPA wire defects); 1 P0 persistent and worsening (skills/verticals seed empty, now TODO 162 minutes).** Distance to v1.0 ship-gate: closer at the backend layer (Claim 5 flipped 🟢 in CLAUDE.md header via #2434); the SPA layer still has the same single load-bearing data-empty defect blocking the demo MP4. Orientation score 63/100 (up from 61 at T0623, down 4 from session high of T0519).

## P0 deltas

### Closed since T0739

1. **Claim 5 backend cold rerun — RESOLVED** via #2434 (`ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms`). All three claim-5 LIVE regression tests cold-passed; CLAUDE.md Claim 5 status flipped 🟡 LIKELY-LIVE → 🟢 LIVE. Closes CLAUDE.md remaining-for-launch item (c).
2. **fetchWorker 400 retry-storm — RESOLVED** via commit `d9455b091`. `fetchWorker` now translates HTTP 400 to `WorkerNotFoundError` same lane as HTTP 404, closing the 15s spinner the camera caught after onboarding navigations (was a Scene 1-2 demo blocker).
3. **/docs subroute fan-out (T0623 F2) + /help placeholder (T0623 F08)** — Both confirmed RESOLVED in this run's whole-site audit. Public-trust marketing pages now have all 10+ subroute placeholders + `/help` 307s to `/docs`. (These were already shipped pre-T0739; T0819 surfaces them as carried-over closures.)

### New P0s this run

1. **`fix-gateway-first-hire-second-call-ghost-worker-not-persisted`** (Onboarding F2) — Gateway returns 201 on second hire per tenant but no row lands in the workers table. UI shows "your teammate isn't visible yet" message that never clears. Customer-trial blocker.
2. **`fix-useteam-query-stuck-loading-spa-fallback-never-resolves`** (Scene 1+2 F1) — `/team` shows infinite 12-skeleton loading state; `useTeam` React Query never resolves to SAMPLE_TEAM fallback. The 5-card SAMPLE deck (Mariah/Peta/Priya/Jamal/Nori) the runbook narrates never renders from the nav-bar "Workers" link. Demo-camera blocker for Scene 1.
3. **`fix-audit-redactor-phone-false-positive-on-action-ids`** (Scene 1+2 F4) — PII redactor regex mangles audit action IDs: 12-digit UUID segments rendered as `[PHONE]` in the table column the camera captures. Customer-visible defect.
4. **`fix-spa-proxy-audit-detail-50x-slower-than-gateway-direct`** (Scene 6 F2 + Scene 1+2 F9) — SPA proxy for `/api/v1/audit/<id>` is 50x slower than gateway-direct; SSR of `/audit/action/<id>` times out >60s. Affects Scene 2 worker-activity drill-in AND Scene 6 audit drill-in.

### Persistent / worsening

- **`fix-gateway-skills-and-verticals-boot-seed-tenant-resolution`** — Still TODO at **162 minutes** across three audit cycles (T0519 → T0623 → T0739 → T0819). Single highest-leverage unlock: closes Scene 3 (Claim 3 verticals fixture path), Scene 4 (Claim 4 skills empty), Scene 5 (Claim 5 workflows runtime). No movement.
- **Audit feed self-traffic pollution** — Degraded 5/5 (T0623) → 10/10 (T0739) → 10/10 (T0819). Every SPA visit adds more rows. Scene 6 drill-in is theatre-empty in a way that's worsening over time.

## MISSION-claim coverage delta

| Claim | T0739 (07:39) | T0819 (08:19) | Δ |
|-------|---------------|---------------|---|
| 1 — Teammate not tool | 🟡 LIKELY-LIVE | 🟡 LIKELY-LIVE | flat (workers-404 + sample fallback unchanged; F2 ghost-worker is new) |
| 2 — Delegation is unit | ❓ NOT-OBSERVABLE-FROM-SPA | ❓ NOT-OBSERVABLE-FROM-SPA | flat (no handoff surface; audit feed still self-traffic) |
| 3 — Platform builds workers | 🟡 LIKELY-LIVE | 🟡 LIKELY-LIVE | flat (verticals empty; Genesis shell present) |
| 4 — Skills shared+learned | 🔴 BROKEN-IN-SPA | 🔴 BROKEN-IN-SPA | flat (skills catalog still empty; the load-bearing P0) |
| 5 — Any-PC execution | ❓ NOT-OBSERVABLE | **🟡 LIKELY-LIVE-via-fixtures** | **improved** (#2434 cold-rerun PASS; templates SSR clickable; runtime workflows still 0) |
| 6 — Multi-channel + multi-tenant | 🟡 LIKELY-LIVE-degraded | 🟡 LIKELY-LIVE-degraded | flat-but-pollution-worsening (audit feed 10/10 self-traffic) |

**SPA-visible 🟢 LIVE claims: 0 of 6.** Demo MP4 remains gated on the F1 P0 brief landing.

## What shipped between T0739 and T0819 (40 min window)

- **#2434** `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms` MERGED (backend Claim 5 → 🟢)
- **#2433** `fix-pre-record-seed-check-script-fail-on-roster-shortfall` in review
- **#2435** `fix-onboarding-dashboard-hire-button-routes-to-step-5` in review
- 7 status/heartbeat commits

Net: 1 P0 backend-resolved, 0 SPA-data-layer P0s resolved.

## Top 3 highest-leverage fix-briefs queued this batch

1. **`fix-useteam-query-stuck-loading-spa-fallback-never-resolves`** (P0) — Closes Scene 1 nav-bar demo-camera blocker. If shipping order is debated, this beats everything else in the new batch because the runbook's first click after onboarding goes through `/team` and the camera catches the 60s+ skeleton.
2. **`fix-spa-proxy-audit-detail-50x-slower-than-gateway-direct`** (P0) — Closes Scene 2 + Scene 6 drill-in. The same defect pollutes both screens; fixing the proxy timeout removes the camera-visible cold-compile blank-state from BOTH scenes in one change.
3. **`fix-audit-redactor-phone-false-positive-on-action-ids`** (P0) — Customer-visible `[PHONE]` token rendered inside audit IDs. Cheap to fix (regex narrowing); high embarrassment cost if it ships uncaught.

## One-line recommendation for the operator

The single bottleneck is still `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` — 162 minutes TODO across 3 audit cycles, blocks 3 of 6 MISSION claims being demo-recordable. Claim it next tick, even if it means deferring the new P0s in this batch.

---

## Push-time amendment (T08:50 Melbourne, post-synthesis)

Between audit-agent finish and this commit landing, the following P0s MERGED:

- **#2436** — `fix-fetchworker-400` + `fix-worker-duty-toggle-E4-h-leak` (closes 2 T0519MEL P0s; commit `b089a24a9`).
- **#2437** — `fix-demo-seed-marketplace-skills-per-tenant-idempotency` (closes the load-bearing `fix-gateway-skills-and-verticals-boot-seed-tenant-resolution` P0 that this DELTA flagged as 162-min unfixed; commit `72c005978`).

**Net result at push time**: #2437 fixes the per-tenant idempotency bug in the demo-seed CLI (was global, silently skipped 2nd tenant) but does NOT auto-populate runtime data. Re-checked at push time: `curl http://127.0.0.1:7400/api/v1/skills?q=competitor` STILL `{"skills":[],"total_count":0}` and `/api/v1/verticals` STILL `{"verticals":[]}`. Claim 4 remains 🔴 BROKEN-IN-SPA until the seed CLI is actually run against the demo tenant. So #2437 is a load-bearing **enabler** for the demo-MP4 unblock but the operator (or a follow-up brief) still has to execute the seed run for the SPA to render real rows.

Standing recommendation revised: the next tick should re-check Scene 4 demo-recordability (skills search now serves real rows) and Scene 3 (verticals still empty per `/api/v1/verticals`).
