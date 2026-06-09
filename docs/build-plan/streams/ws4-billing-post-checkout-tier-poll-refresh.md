# ws4-billing-post-checkout-tier-poll-refresh

**Owner**: codex.
**Authored against**: origin/main `eddcab780` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ws4-billing-post-checkout-tier-poll-refresh`.

## Why (WS4 billing — VERIFIED gap 2026-06-09)
After Stripe Checkout succeeds it redirects the buyer to
`${origin}/billing?upgrade=ok` (the literal `success_url` built at
`web/lib/billing/use-checkout.ts:39`). But the `/billing` page does NOT
react to `?upgrade=ok` — it fires a single `useQuery(["billing","usage"])`
(`web/app/(app)/billing/page.tsx:96-99`) that resolves the tier ONCE on
mount. The Stripe webhook that flips Free→Pro
(`POST /api/v1/billing/webhook` → `argo_billing::tier_store::record_event`,
`crates/argo-cli/src/handlers/gateway/billing.rs:120-158`) is **async** and
typically lands a beat after the redirect. Net effect: a just-paid customer
returns to `/billing`, still sees their OLD plan ("Current tier: Free"), and
has no signal anything is happening until they manually refresh. That is the
single worst moment for an "I paid and nothing changed" experience — it is
the exact instant money changed hands.

VERIFIED (grep against `eddcab780`):
- `rg -n 'upgrade=ok|invalidateQueries|refetch|poll' web/app/(app)/billing/page.tsx` → **ZERO hits**.
- `rg -n 'upgrade=ok|invalidateQueries|refetch|poll' web/app/(app)/billing/subscribe/page.tsx` → **ZERO hits** (subscribe only kicks off checkout, never handles the return).
- `web/lib/billing/use-checkout.ts:39` → `success_url: ... ${origin()}/billing?upgrade=ok` (confirms the return URL + query param).
- `web/lib/api/usage.ts:19-33` → `UsageResponse.tier: "free" | "pro" | "enterprise"`, fetched from `GET /api/v1/tenant/usage`.

## Grep-before-write
- `rg -n 'upgrade=ok|useSearchParams|invalidateQueries|refetch|refetchInterval|poll' web/app/(app)/billing/page.tsx` — confirm none exist before adding.
- `rg -n 'useQuery|queryKey|fetchTenantUsage|UsageResponse' web/app/(app)/billing/page.tsx web/lib/api/usage.ts` — the existing query to extend (do NOT add a parallel fetch path).
- `rg 'pub (struct|enum|fn) ' web/` is N/A (TypeScript). TS equivalent — confirm no parallel hook is introduced: `rg -n 'export function use.*Tier|usePostCheckout|useUpgradePoll' web/lib web/app` → **must stay ZERO** (extend the existing `useQuery`, do not author a new exported hook unless it is a thin wrapper around the same `queryKey`).
- Reuse `useSearchParams` from `next/navigation` (already imported in `subscribe/page.tsx:15`); reuse `useQueryClient`/`refetch` from `@tanstack/react-query` (already a dep — `page.tsx:27`).

## Fix (additive, name consumers)
1. In `web/app/(app)/billing/page.tsx`, read `useSearchParams()` and detect `upgrade === "ok"`.
2. While `upgrade=ok` AND the fetched `data.tier` is still the pre-upgrade tier (treat `free` as "not yet upgraded" for the common Free→Pro/Enterprise path), **poll the existing usage query** with bounded backoff — e.g. enable `refetchInterval` on the `["billing","usage"]` `useQuery` (or call `refetch()` on a timer) for a few attempts (suggest ~5 attempts, ~1.5s→6s capped, total ≤ ~20s) until the tier flips, then stop polling. Reuse the SAME `queryKey`/`fetchTenantUsage` — no parallel fetch.
3. Meanwhile render an honest, non-blocking "Processing your upgrade…" state on the plan card (not a fake "Pro active" — NO THEATER, DEC-029). When the tier flips, drop the processing state and show the new plan normally.
4. If polling exhausts without a flip, fall back to a calm "Still processing — refresh in a moment" note (NOT an error toast; the webhook may just be slow). Do not block the page.
5. Consumer: the `/billing` plan card region in `page.tsx` (the `PLANS.map(...)` block, ~line 139) renders the processing/flip state; no new route, no new API module, no backend change.

Keep the whole change additive and small (one page file + its test). No edits to `use-checkout.ts`, the gateway, or `usage.ts`.

## Acceptance (truth-bounded; < 500 LOC)
1. **vitest** (new, co-located e.g. `web/app/(app)/billing/page.upgrade-poll.test.tsx` or extend an existing billing test): mock `fetchTenantUsage` / the `/api/v1/tenant/usage` query so it returns `tier: "free"` on the first call(s) then `tier: "pro"`; render `/billing` with `?upgrade=ok` (mock `useSearchParams`). Assert: (a) the usage query is **refetched more than once** on `?upgrade=ok` (the page polls — spy on the query fn / advance fake timers), and (b) once the mock flips to `pro`, the page renders the **new plan** (Pro) and the "Processing your upgrade…" state is gone.
2. Negative/honest path: with `?upgrade=ok` and the mock NEVER flipping, assert polling **stops after the bounded attempt cap** (no infinite refetch loop) and the calm "still processing" fallback renders — NOT a thrown error.
3. Control: WITHOUT `?upgrade=ok`, assert the page does its single existing fetch and does NOT poll (no behavior change for normal visits).
4. `pnpm --filter web test` (or the repo's vitest invocation) green on the touched test; `pnpm --filter web lint`/typecheck clean on the touched file.

## On ship (same commit)
- Update the `FEATURE_MATRIX.md` billing row note to record post-checkout tier auto-refresh (status badge + commit SHA + `web/app/(app)/billing/page.tsx`). Do NOT regenerate/commit `docs/skycot/feature-matrix.html` (DEC-043 — it is gitignored).

## Effort
S (one page file + one vitest; pure SPA, additive, no backend).
