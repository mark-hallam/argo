# UX e2e audit — T23-09 delta vs T11-00

**This run**: 2026-05-25T13:09Z (23:09 Melbourne) — 5-agent parallel sweep + synthesis.
**Prior run**: 2026-05-25T11:00Z (T11-00) — 4-agent sweep + synthesis batch (28 root-cause briefs landed).

## Headline

**23 P0s → 26 P0s.** Net +3 P0s on the punch-list, but most of the +3 come from **deeper coverage in this run** (5 agents incl. whole-site rollup vs. 4 in T11-00), not from regressions on previously-clean ground.

**11 P0 root-cause clusters persist** unchanged from T11-00 (workflows MOCK pill, competitor-pricing search, customer_research_followup, Rio/Sam non-canon workers, no Install button, gateway team API has Maya not 5, no Run-now button, /workers/<id>/responsibilities 404, /audit replay fabrication, audit list/detail ID mismatch, audit "unknown" worker). All of these are in flight as IN_REVIEW / CLAIMED / BLOCKED / TODO in BUILD_STATUS.md under the T11-00 batch.

**~6-8 P0 clusters appear closed or downgraded** since T11-00 (notably the charter-object crash on /workers/<id>/overview, which T11-00 surfaced 4× — it does not reappear in T23-09's P0 list, suggesting the brief landed or the symptom is now masked behind the new "Worker not found" session-token race).

**9 net-new P0s** authored as T23-09 fix-briefs:
1. Gateway CORS missing on real responses (onboarding F01 + scene-6 F1)
2. Worker persona hardcoded frontend (onboarding F02)
3. Onboarding vertical templates ≠ /workers starter prompts (onboarding F04)
4. `/api/v1/audit` cross-tenant leak for legacy rows (scene-6 F5) — **CRITICAL SECURITY**
5. Section labelled "Responsibilities" not "Schedules" (scene-1-2 F5) — UI vocab regression
6. Worker-detail "Worker not found" — session-token race (scene-1-2 F6)
7. demo-mint hardcodes `needs_onboarding:true`; bounces Scene 1 entry (scene-1-2 F7)
8. SPA theme toggle broken (whole-site F1)
9. /settings/members hydration mismatch → unstyled page (whole-site F3)

The CORS finding (F01) is the loudest — almost every cross-origin call from the SPA fails. The audit list legacy-rows leak (scene-6 F5) is the most dangerous (cross-tenant data visibility).

## P0 delta

| Status vs T11-00 | Count | Examples |
|---|---|---|
| Persistent (root cause unchanged) | ~11 | workflows MOCK pill, competitor-pricing search empty, /skills no Install button, /audit replay fabrication, audit list/detail ID mismatch, no Run-now button |
| Closed / downgraded | ~6-8 | charter-object crash on /workers/<id>/overview (no longer top-of-list), team-hero hardcoded actions/hours (not in T23-09 P0 list) |
| Net-new | 9 | gateway CORS missing, audit list cross-tenant legacy-rows leak, /settings/members unstyled, SPA theme toggle broken, demo-mint bounces Scene 1, worker-detail session-token race, Schedules-vs-Responsibilities vocab regression |

## 6-MISSION-claim coverage delta

Comparing the whole-site MISSION-claim rollups. **Both runs evaluate the SPA at MVP scope, but T23-09's "rollup" agent narrowed scope to non-Scene pages**, so direct comparison is partial. The four sister agents cover Scenes 1-6.

| Claim | T11-00 | T23-09 | Direction |
|---|---|---|---|
| 1 — Teammate, not tool | 🟡 PARTIAL | 🟢 WORKS (marketing scope) | ⬆ — but caveated to marketing scope only; in-app /workers verdict owned by sister-agent (still 🟡 there) |
| 2 — Delegation is the unit | 🟡 PARTIAL | 🟡 PARTIAL | ↔ — described in marketing copy, still not visibly invoked |
| 3 — Platform builds workers | 🔴 NOT SUBSTANTIATED | 🟡 PARTIAL | ⬆ — hire-template grid + brand-loud /team hero make it feel real; cohorts empty-state hurts it (F6) |
| 4 — Skills shared + learned | 🔴 NOT SUBSTANTIATED | 🟢 WORKS (workflows/templates scope) | ⬆ — A1-marketplace pattern proven at /workflows/templates; /skills itself still not audited at MVP (sister-agent scope) |
| 5 — Any-PC execution | 🔴 NOT SUBSTANTIATED | 🔴 INVISIBLE | ↔ — 29 MCP integrations listed in Settings → Connectors, but computer-use not surfaced |
| 6 — Multi-channel + multi-tenant | 🟡 PARTIAL | 🟡 PARTIAL | ↔ — multi-channel told well; multi-tenant actively undermined by F21 (workspace pill always "Personal workspace") |

**Net**: 2 claims moved ⬆ (1, 3, 4 — counting 4 as the meaningful jump), 0 moved ⬇. Claim 5 unchanged (computer-use invisible in SPA — operator-only macOS perms still pending per CLAUDE.md item (c)).

Distance to ship-gate: **shorter than T11-00**, but the 11 persistent P0s + 9 net-new P0s still block recording. The 5 highest-leverage net-new briefs in the T23-09 batch (audit cross-tenant leak, gateway CORS, theme-toggle broken, /team empty-state-despite-Maya, worker-detail session-token race) are the gating items along with the still-open T11-00 batch's "no Run-now button" + "/workers/<id>/responsibilities missing" + "/skills no Install button" trio.

## Top 3 highest-leverage net-new fix-briefs

1. **[fix-audit-list-row-in-tenant-scope-legacy-leak](../../../build-plan/streams/fix-audit-list-row-in-tenant-scope-legacy-leak.md)** — Cross-tenant confidentiality leak in `/api/v1/audit` list path. Critical security; not a demo-blocker but a ship-blocker. Should leap to top of next worker pickup.
2. **[fix-gateway-workflows-and-skills-marketplace-endpoints-404](../../../build-plan/streams/fix-gateway-workflows-and-skills-marketplace-endpoints-404.md)** — Top-level gateway endpoints `/api/v1/skills`, `/api/v1/skills/marketplace`, `/api/v1/workflows`, `/api/v1/workflows/list`, `/api/v1/skill-library/list` all 404. Single-fix wires multiple claim-3/4 surfaces.
3. **[fix-demo-mint-needs-onboarding-flag-bounces-scene-1](../../../build-plan/streams/fix-demo-mint-needs-onboarding-flag-bounces-scene-1.md)** — demo-mint hardcodes `needs_onboarding:true`; Scene 1 entry navigation bounces to `/onboarding/pick-vertical` instead of `/workers`. Tiny fix; unblocks the camera's first move.

Honourable mention: **[fix-team-page-empty-state-despite-gateway-worker](../../../build-plan/streams/fix-team-page-empty-state-despite-gateway-worker.md)** — /team renders empty-state grid even when gateway returns workers. Scene 1's first beat unrecordable. Likely a fetch-error swallow or a tenant-scope mismatch.

## Methodology notes for next tick

- Continue 5-agent parallel structure — the whole-site rollup catches issues the four scene-specific agents miss (theme toggle, settings/members hydration, marketing nav fragments).
- The synthesis agent de-dupe pass against existing fix-briefs is now load-bearing — 24 of ~60 findings were already brief'd at T11-00 and got skipped cleanly.
- Next 4-hourly tick should expect the persistent T11-00 P0 count to drop as in-flight workers ship PRs.
