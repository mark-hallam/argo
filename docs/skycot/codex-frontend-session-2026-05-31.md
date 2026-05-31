# Codex Frontend Session - 2026-05-31

Owner: Codex frontend session.
Lane: Next.js SPA under `web/`, excluding `/genesis`, `/pitch`, `/gtm`, `/marketing` and their owned clients.

## Rules I Am Following

- VERIFIED: Read `CLAUDE.md`, `docs/skycot/brand/DESIGN_SYSTEM.md`, `docs/skycot/decisions/DEC-029-truth-bounded-status-reporting.md`, `docs/skycot/decisions/DEC-007-no-theater-in-demo.md`, `docs/build-plan/FEATURE_MATRIX.md`, the locked 2026-05-18 design inventory, and `docs/skycot/marketing/DIFFERENTIATORS.md`.
- VERIFIED: This clone must not start `next dev`, run `next build`, run any server, run Cargo, compile Rust, touch `:3000`, touch `:7400`, touch `~/.argo/*`, edit `BUILD_STATUS.md`, or edit swarm/worker protocol files.
- VERIFIED: Rust gateway files may be read only for endpoint discovery. All frontend status below is truth-bounded to static source and Node-only verification from this clone.
- VERIFIED: Brand is locked to Audiowide/Inter and the purple-to-pink gradient `#8B5CF6 -> #EC4899`. Purple-to-cyan is deprecated; do not add it.

## Verification Ledger

- VERIFIED: `git status --short --branch` reported `## main...origin/main` before this audit file was added.
- VERIFIED: `pnpm exec tsc --noEmit` in `web/` fails with six baseline errors:
  - `app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx(39,9)`: missing `substrate`.
  - `lib/sample-workflows.ts(239,3)`: missing `authorType`.
  - `lib/sample-workflows.ts(251,3)`: missing `authorType`.
  - `lib/sample-workflows.ts(263,3)`: missing `authorType`.
  - `tests/settings-workspace-tenant-id.test.tsx(154,51)`: `null` not assignable to `string`.
  - `tests/settings-workspace-tenant-id.test.tsx(194,51)`: `null` not assignable to `string`.
- VERIFIED: `web/node_modules` originally had stale pnpm links into `/private/tmp/...`; removing `web/node_modules` and running `pnpm install --frozen-lockfile` restored local TypeScript execution. No source or lockfile change was made for that relink.
- UNCONFIRMED: Live gateway runtime behavior. This clone is intentionally not allowed to touch the running gateway, so endpoint availability is based on source discovery only.

## Surface Audit

| Surface | Demo-live data status | Endpoint/source evidence | Locked design status | Notes |
| --- | --- | --- | --- | --- |
| `/team` | VERIFIED source first attempts gateway in demo-live, but still falls back to `SAMPLE_TEAM` on gateway failure in demo mode. | `web/lib/api/team.ts`; tests explicitly expect `SAMPLE_TEAM` fallback on 404/5xx/malformed/timeout. | VERIFIED partial/full C2 direction in source via `BrandLoudHero` and team card grid. | P0 no-theater risk for recording if gateway fails. Needs product decision or strict demo-live error state. |
| `/workers` | VERIFIED redirects to `/team` when workers exist; still has local empty/just-hired presentation. | `web/app/(app)/workers/page.tsx`, `web/lib/api/workers.ts`. | VERIFIED partial: locked Workers C2 page exists in design system, but canonical in-product roster is now `/team`. | Audit whether `/workers` should remain redirect-only or receive locked implementation. |
| `/workers/[workerId]` | VERIFIED uses live worker overview/activity/memory/skills/trust/settings clients where available; several clients have demo-live guards. | Feature Matrix rows plus `web/lib/api/worker-*`. | VERIFIED worker-detail C2 personality lock exists. | P1 inspect schedule/escalation sub-surfaces for fallback behavior. |
| `/workers/[workerId]/escalation` | UNCONFIRMED backend live endpoint for escalation events. | Static scan found sample trust/history usage; no escalation gateway route verified yet. | UNCONFIRMED design parity. | P1 either wire endpoint if found or label honestly. |
| `/approvals` | VERIFIED main approvals client uses canonical `isAuthBypassed() && !isDemoLive()` gate. | `web/lib/api/approvals.ts`. | VERIFIED ops-dashboard direction implemented at least partially. | Elevated-view sub-action uses separate client with mock-bypass gap listed below. |
| `/audit` | VERIFIED main audit client uses canonical demo-live gate. | `web/lib/api/audit.ts`, `web/lib/api/audit-detail.ts`. | VERIFIED partial: current UI is timeline/feed; locked B3 enterprise/block-explorer needs visual parity pass. | Elevated-view and audit-export clients have demo-live mock fallback risk. |
| Elevated view | VERIFIED source has live gateway route; client currently mock-grants on auth bypass and 404 without checking demo-live. | `crates/argo-cli/src/handlers/gateway/routes_elevated_view.rs`; `web/lib/api/elevated-view.ts`. | N/A shared action. | P0 de-theatre: canonical gate plus tests. |
| Audit export | VERIFIED client returns mock preview/generate on bare auth bypass. | `web/lib/api/audit-export.ts`. | N/A shared action. | P1 determine whether source endpoint exists; if not, label export as partial/honest. |
| `/cohorts` | VERIFIED feature matrix says live; page code says locked D3 magazine tiles. | `web/app/(app)/cohorts/page.tsx`, `web/lib/api/cohorts.ts`. | VERIFIED implemented direction. | P2 visual/mobile pass only unless tests reveal gaps. |
| `/skills` | VERIFIED main skills client uses canonical gate. | `web/lib/api/skills.ts`. | VERIFIED A1 marketplace direction implemented in source/CSS. | P0/P1 subflows: Ghostwriter and Skill IR clients still bypass/fallback to mocks in demo-live. |
| Ghostwriter | VERIFIED source has live gateway route; client returns mock draft on bare auth bypass. | `crates/argo-cli/src/handlers/gateway/routes_ghostwriter.rs`; `web/lib/api/ghostwriter.ts`. | N/A shared action. | P0 de-theatre: canonical gate plus tests. |
| Skill IR read/save/export/import | VERIFIED source has live gateway routes; clients use bare auth-bypass fallback. | `crates/argo-cli/src/handlers/gateway/routes_skill_ir.rs`; `web/lib/api/skill-ir.ts`, `skill-ir-save.ts`, `skill-ir-export.ts`. | N/A shared action. | P0 de-theatre and fix 404/sample fallback in demo-live. |
| `/workflows` | VERIFIED source has live workflow clients; some clients intentionally seed for demo-live. | `web/lib/api/workflows.ts`, `workflow-runs.ts`. | UNCONFIRMED: design system references 2026-05-21 workflow goldens, but that directory is absent in this checkout. | P1/P2 replace deprecated cyan/purple gradients and token literals in builder. |
| `/orchestration` | VERIFIED client helper includes demo-live opt-out in auth-bypass predicate. | `web/lib/api/orchestration.ts`. | UNCONFIRMED locked design parity. | P2 tests/visual pass. |
| `/settings/*` | MIXED. Knowledge/members/security/connectors listed live in Feature Matrix; API keys and provider keys are scaffold/no endpoint found; notify channels uses an absent `/api/doctor/channels` route with fallback registry. | `web/lib/api/settings.ts`; `api-keys.ts`; `provider-keys.ts`; `web/app/(app)/settings/notify-channels/page.tsx`; `web/lib/notify-channels/registry.ts`. | VERIFIED settings top-tabs lock exists; source partially implements. | P0 keep API/provider keys honest; P1 backend list for `/api/v1/api-keys`, `/api/v1/providers/keys`, and `/api/doctor/channels`. |
| `/onboarding` | VERIFIED partial in Feature Matrix; source includes demo-mode provider/provision/first-hire skips. | `web/lib/onboarding/*`, onboarding pages. | VERIFIED F1 editorial-center lock implemented. | P1 audit copy to ensure skips are honestly marked, not dressed as live. |
| `/billing` and `/settings/billing` | UNCONFIRMED live payment state. | Not fully inspected yet. | UNCONFIRMED. | P1 audit needed. |
| `/verticals` | VERIFIED client has canonical demo-live gate. | `web/lib/api/verticals.ts`. | UNCONFIRMED locked design parity. | P2 visual/brand pass. |
| Public landing | VERIFIED locked D3 implementation exists; four differentiators are not surfaced as a clear additive section. | `web/app/landing/page.tsx`, `page.css`; `docs/skycot/marketing/DIFFERENTIATORS.md`. | VERIFIED D3 landing lock implemented. | P0/P1 add differentiators without redesign. |
| Public customer pages | VERIFIED placeholder customer stories have `dataPending: true`; landing/customers copy presents them as customer proof. | `web/lib/content/customers.ts`; `web/app/customers/*`; `web/app/landing/page.tsx`. | UNCONFIRMED locked design parity. | P0 no-theater risk on public pages: relabel/hide placeholders or source real proof. |
| Public compare/about/pricing/security/docs/for pages | UNCONFIRMED data/live status. | Routes enumerated; not fully inspected. | UNCONFIRMED. | P2 audit copy and brand tokens after P0 gaps. |

## Endpoint Findings For Backend Coordination

- VERIFIED source endpoint exists: `POST /api/v1/skills/draft-from-prompt`.
- VERIFIED source endpoint exists: `GET/POST /api/v1/skills/<skillId>/ir`, `POST /import`, `POST /export`, and version endpoints.
- VERIFIED source endpoint exists: `POST /api/v1/audit/elevated-view-request`, `GET /api/v1/audit/elevated-view-grants`, `DELETE /api/v1/audit/elevated-view-grants/<id>`.
- VERIFIED source endpoint exists: worker duties classify/preview/commit and duty update/delete routes.
- VERIFIED source endpoint exists: worker policies GET/PATCH route.
- VERIFIED source endpoint not found in gateway scan: `/api/v1/api-keys`.
- VERIFIED source endpoint not found in gateway scan: `/api/v1/providers/keys`.
- VERIFIED source endpoint not found in gateway scan: `/api/doctor/channels`.
- UNCONFIRMED runtime: none of the above were called against `:7400` from this clone.

## Brand And Design Findings

- VERIFIED: `web/app/globals.css` exposes the runtime brand tokens and the locked purple-to-pink gradient.
- VERIFIED: Existing source still contains deprecated cyan/purple-cyan debt in workflow builder surfaces, customer gradients, compare/changelog/customer pages, and one off-limits GTM page. The GTM occurrence is Mac-owned and will not be touched by this session.
- VERIFIED: `docs/skycot/design-explorations/2026-05-21/` is absent in this checkout even though the design system references workflow goldens there.
- UNCONFIRMED: Full JSX parity with every locked 2026-05-18 page design. Static source comments indicate several are implemented, but visual parity requires targeted screenshots in the primary running SPA, which this clone cannot start.

## Prioritized Worklist

1. P0 - Fix TypeScript baseline:
   - Add missing `substrate` to `SubWorkflowPicker.test.tsx`.
   - Add missing `authorType` fields in `lib/sample-workflows.ts`.
   - Fix the two settings tenant-id test `null` type errors.
2. P0 - De-theatre demo-live clients with verified source endpoints:
   - `web/lib/api/elevated-view.ts`
   - `web/lib/api/ghostwriter.ts`
   - `web/lib/api/skill-ir.ts`
   - `web/lib/api/skill-ir-save.ts`
   - `web/lib/api/skill-ir-export.ts`
3. P0 - Stop public placeholder customer stories from reading as real customer proof, or label them honestly.
4. P1 - Add the four verified differentiators to the existing locked landing page as an additive, in-brand section.
5. P1 - Keep API keys, provider keys, and notify channels honest until backend endpoints exist; list `/api/v1/api-keys`, `/api/v1/providers/keys`, and `/api/doctor/channels` for backend.
6. P1 - Audit duties/worker-policies/audit-export fallback paths and make demo-live strict where live source endpoints exist.
7. P2 - Replace deprecated cyan/purple-cyan and literal color debt inside owned frontend surfaces, excluding Mac-owned `/gtm`.
8. P2 - Locked design parity pass for audit B3, workers/team C2, settings D top-tabs, and workflows once workflow goldens are available.

## Progress Log

- VERIFIED 2026-05-31: Phase 1 audit started; route inventory, API gate scan, endpoint source scan, brand debt scan, locked-design inventory, and TypeScript baseline completed from this clone.
- VERIFIED 2026-05-31: Fixed the six TypeScript baseline errors in workflow fixtures and tenant-id tests. `pnpm exec tsc --noEmit` now exits 0 in `web/`.
- VERIFIED 2026-05-31: Touched-file tests passed: `pnpm exec vitest run 'app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx' tests/settings-workspace-tenant-id.test.tsx` -> 2 files / 7 tests passed.
- VERIFIED 2026-05-31: De-theatred demo-live auth-bypass/mock fallback for elevated view, Ghostwriter, Skill IR read, Skill IR save, and Skill IR export/import. `pnpm exec tsc --noEmit` exits 0; `pnpm exec vitest run lib/api/demo-live-mode.test.ts` -> 1 file / 24 tests passed.
- UNCONFIRMED 2026-05-31: Live gateway responses for those endpoints remain untested in this clone by instruction; coverage proves frontend routing behavior only.
- VERIFIED 2026-05-31: Added the four verified differentiators to the locked landing page without redesigning it, and relabeled seeded public customer stories as representative profiles instead of real customer proof. `pnpm exec tsc --noEmit` exits 0; `pnpm exec vitest run lib/api/demo-live-mode.test.ts 'app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx' tests/settings-workspace-tenant-id.test.tsx` -> 3 files / 31 tests passed.
- VERIFIED 2026-05-31: Updated `docs/build-plan/FEATURE_MATRIX.md` for the shipped frontend changes and regenerated `docs/skycot/feature-matrix.html` with `python3 scripts/render-feature-matrix.py` (`wrote ~/code/argo-codex/docs/skycot/feature-matrix.html`).
- VERIFIED 2026-05-31: Full frontend verification is green after repairing stale fixture/test expectations found by the first full run. `pnpm exec tsc --noEmit` exits 0; `pnpm exec vitest run` -> 312 files / 2400 tests passed.
- VERIFIED 2026-05-31: Continued de-theatre pass for remaining camera-visible frontend surfaces with live gateway endpoints. Demo-live now disables 404/TypeError sample fallbacks for `/team`, worker-detail overview/activity/memory/skills/settings/policies/trust, worker read/pause/resume/hire, and audit export preview/generate. Missing live rows now surface as honest not-found/gateway errors instead of seeded persona rows, mock audit bundle counts, or synthetic mutation success.
- VERIFIED 2026-05-31: Kept no-endpoint key surfaces honest. `/settings/api-keys` and `/settings/providers` now call the real endpoints under demo-live and render an unavailable state with create/add controls disabled when `/api/v1/api-keys` or `/api/v1/providers/keys` is absent; plain local auth-bypass still keeps fixture/mock-store behavior for offline dev.
- VERIFIED 2026-05-31: Node evidence for this pass: `pnpm exec vitest run lib/api/demo-live-mode.test.ts lib/api/api-keys.test.ts lib/api/provider-keys.test.ts lib/api/team.test.ts lib/api/worker-trust.test.ts 'app/(app)/settings/providers/page.test.tsx' 'app/(app)/settings/api-keys/page.test.tsx'` -> 7 files / 72 tests passed; final `pnpm exec tsc --noEmit` exits 0; final `pnpm exec vitest run` -> 314 files / 2413 tests passed.
- VERIFIED 2026-05-31: Updated `docs/build-plan/FEATURE_MATRIX.md` for `/team`, worker-detail, `/settings api-keys + provider-keys`, and audit export; regenerated `docs/skycot/feature-matrix.html` with `python3 scripts/render-feature-matrix.py` (`wrote ~/code/argo-codex/docs/skycot/feature-matrix.html`).
- UNCONFIRMED 2026-05-31: Live gateway/runtime browser behavior for the changed surfaces remains untested from this Codex clone by instruction (`next dev`/`:3000`/`:7400` off-limits). Backend still needs endpoints for `/api/v1/api-keys`, `/api/v1/providers/keys`, and `/api/doctor/channels`.
