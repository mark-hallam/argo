# web-wire-escalation-panel-to-live-read

**Owner**: claude.
**Authored against**: origin/main `eddcab780` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-wire-escalation-panel-to-live-read`.

## Why (NO-THEATER leak on a SAFETY surface, VERIFIED on origin/main `eddcab780`)
The worker-detail **Escalation** tab renders FABRICATED escalation rules + history while a REAL backend read shipped but is never consumed. Escalation is a SAFETY surface — fabricated rules misrepresent what the teammate will actually pause-and-escalate on, so the leak is worse than a cosmetic one (DEC-007 / DEC-029).
- `web/app/(app)/workers/[workerId]/escalation/page.tsx:9` imports `findSampleTrustHistory` from `@/lib/sample-worker-trust-history` and (line 40-177) hard-codes `SEEDS` / `DEFAULT_SEED` escalation rules + history per worker id. The page header comment (lines 11-20) explicitly admits: "No gateway dependency for v1 — once the audit-row feed lands, the history table will swap for a real query against `GET /api/v1/workers/<id>/escalations`."
- **That endpoint now EXISTS.** `crates/argo-cli/src/handlers/gateway/routes_worker_escalations.rs:64` `list_response` serves `GET /api/v1/workers/<id>/escalations`, returning `{ "rules": [ { action, condition, routes_to, channel? }, ... ], "is_default": <bool> }` — persisted rules from `.skycot/worker-escalations/<worker_id>.toml`, with an **empty list + `is_default: true`** when the operator has authored none (`routes_worker_escalations.rs:96-103`). It is cross-tenant-hidden (404) and matches the SPA `EscalationRule` contract (`web/components/skycot/workers/EscalationMatrix.tsx`). (The PATCH sibling writes + audits.)
- But `web/lib/api/` has NO `escalations` module consuming it — `rg -rln 'escalations' web/lib/api` returns nothing. The page is fully unwired.
- **Endpoint-scope note (verified):** the live GET exposes the persisted RULE SET (+ `is_default`), not a per-event escalation HISTORY feed, and not a trust-over-time series. So this brief wires the **rules** (the `CurrentRulesCard` + `EscalationMatrix`) to the live read and honest-empty-states the **history** table + trust chart (rather than showing fabricated past events). Inventing a history feed is out of scope.

## Grep-before-write
- `rg -n 'findSampleTrustHistory|SEEDS|DEFAULT_SEED|getSeed' web/app/\(app\)/workers/\[workerId\]/escalation/page.tsx` — the fabricated sources to displace.
- `rg -n 'list_response|is_default|escalations' crates/argo-cli/src/handlers/gateway/routes_worker_escalations.rs` — confirms the live GET shape (`rules` + `is_default`) before wiring; this is the endpoint the page comment already names.
- `rg -rln 'escalations' web/lib/api` — proves no `escalations.ts` exists yet (so a NEW module is justified, not a parallel surface).
- `rg -n 'export function hasRealSession|export function isDemoLive|export function isGatewayAuthBypassed' web/lib/skycot/demo-mode.ts` — the canonical demo gate (note: `isGatewayAuthBypassed`, NOT `isAuthBypassed`). Mirror the duties module's `mockFallbackAllowed()` pattern (`!hasRealSession()`, `web/lib/api/duties.ts:25`) so a real session / demo-live never sees the sample.
- `rg -n 'export interface EscalationRule|EscalationMatrix|TrustOverTimeChart' web/components/skycot/workers/*.tsx` — the consumer components whose props the live shape must satisfy.
- New surface: `web/lib/api/escalations.ts` (+ optional `web/lib/hooks/use-worker-escalations.ts`). `rg 'export (function|const) listEscalations' web/lib` / `rg 'use-worker-escalations' web/lib/hooks` — confirm no collision before adding.

## Fix (additive; names its consumer)
1. Add `web/lib/api/escalations.ts` exporting `listEscalations(workerId, fallback?)` that GETs `/api/v1/workers/<id>/escalations` via the shared `apiGet` client and parses `{ rules, is_default }` into the SPA `EscalationRule[]`. Apply the canonical NO-THEATER gate exactly like `web/lib/api/duties.ts`: when `hasRealSession()` (real signed-in) OR `isDemoLive()` is true, a 404 / malformed body throws an honest error and an empty/`is_default` rule set returns `[]` (honest empty); only anonymous-dev (no session, not demo-live) falls back to the caller-supplied sample rules. Re-use the duties module's `mockFallbackAllowed` pattern; do not introduce a parallel gate.
2. In `escalation/page.tsx`, replace the `getSeed(workerId)` rules source with `listEscalations(workerId, () => getSeed(workerId).rules)` (a thin `useQuery` or a `use-worker-escalations` hook). Feed the live rules into both `CurrentRulesCard` and `EscalationMatrix`. Drop `findSampleTrustHistory` / `SEEDS.history` from the live path.
3. History + trust-chart honesty: because the live GET does not expose a per-event history feed, render the `HistoryCard` **empty state** (it already has one: `escalation-history-empty`, "No escalations recorded...") and render the `TrustOverTimeChart` with `points={[]}` (or hide it) when in real-session/demo-live mode. Keep the sample history + trust series ONLY for anonymous-dev (no session, not demo-live), matching the duties pattern. Do NOT fabricate history events for a real/demo-live operator.
4. Preserve the page's loading / error tabpanel structure + all existing `data-testid`s (`worker-tab-panel-escalation`, `escalation-current-rules`, `escalation-history`, `escalation-rule`). The only NEW consumer of `listEscalations` is this page (+ optional hook).

## Acceptance (truth-bounded)
1. Vitest `web/lib/api/escalations.test.ts`: mock `@/lib/api/client` `apiGet` + `@/lib/skycot/demo-mode`. (a) real session / demo-live + gateway `{rules:[...], is_default:false}` → returns the LIVE rules; (b) real session + `{rules:[], is_default:true}` → returns `[]` (honest empty), NOT the sample; (c) real session + 404 → throws (no sample leak); (d) anonymous dev + 404 → falls back to the supplied sample rules.
2. Vitest (component, extend the escalation page test or add one): in real-session/demo-live mode the page renders the LIVE rules (assert a gateway-returned `condition` appears and a SAMPLE-only `condition` does NOT) and the honest empty history state; anonymous-dev renders the sample.
3. `rg -rln 'listEscalations' web/app web/lib/hooks` returns the page/hook (the leak-closing wire exists); the page no longer imports `findSampleTrustHistory` on its real path.
4. `cd web && npx vitest run` green for touched test files; `npx tsc --noEmit` clean on touched files; no file > 500 LOC.

## On ship (same commit)
- Flip the Escalation-panel-wiring row in `docs/build-plan/FEATURE_MATRIX.md` to ✅ Live with this commit SHA + `web/lib/api/escalations.ts` + `web/app/(app)/workers/[workerId]/escalation/page.tsx` (add the row if absent; do NOT regenerate `docs/skycot/feature-matrix.html` — GITIGNORED per DEC-043).
- Note in the commit body that it closes the NO-THEATER Escalation-tab sample-data leak (DEC-007/DEC-029) on a safety surface by consuming the live `GET /api/v1/workers/<id>/escalations` read.

## Effort
S–M (one new ~80-LOC api module + one page rewire + 2 vitest files; live read already shipped, so no backend work — the only judgement is honest-empty-stating the history/trust-chart that the GET does not feed).
