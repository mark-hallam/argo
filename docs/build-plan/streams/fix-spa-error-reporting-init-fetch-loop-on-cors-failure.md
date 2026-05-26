# fix-spa-error-reporting-init-fetch-loop-on-cors-failure

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-spa-error-reporting-init-fetch-loop-on-cors-failure`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/onboarding-hire-an-ai-teammate.md` F16 [P1] + `scene-1-and-2-claim-1-and-2.md` methodology note + `scene-6-claim-6-trust-and-approvals.md` methodology note + `whole-site-mvp-claims-coverage.md` methodology note.

🟠 **P1 — Browser-side runaway fetch loop on `/api/v1/meta/error-reporting` wedges the renderer within ~30s. The SPA's global `window.error` handler is itself triggering a fetch that fails because of CORS; the failure fires another error event; the handler refetches. The audit team's Claude Preview MCP wedged in 4 of the 5 audits in this run because of this loop.**

## Root cause

`web/lib/error-reporting/init.ts:53` (verify path — line range per audit) calls `fetch('/api/v1/meta/error-reporting')` from a global `window.error` listener that is re-attached on every page mount. Two coupled defects:

1. The gateway's `/api/v1/meta/error-reporting` response is missing CORS headers (covered upstream by `fix-gateway-cors-localhost-127-and-preview-proxy-origins`), so the fetch resolves with `net::ERR_FAILED` in the browser.
2. The fetch failure throws — and the throw fires `window.error` — which the SPA's global handler interprets as a new client-side error → another POST attempt → loop.

The audits observed `/landing → /skills → /workers → /team → /approvals` cycling at <100ms intervals, each mount firing 4-6 gateway fetches that all fail. After ~30 seconds the preview renderer is unresponsive.

This is the same shape as the historic "perpetual loading" bug class. CORS misconfig + an error-handler that itself fires fetches is a positive-feedback loop. Even after CORS is fixed (separate brief), the handler should not be allowed to recurse on its own failures.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F16 [P1] — Browser-side runaway fetch loop on the `meta/error-reporting` endpoint wedges the renderer
- scene-1-and-2-claim-1-and-2 methodology note — Claude Preview MCP wedged after first navigation
- scene-6-claim-6-trust-and-approvals methodology note — Preview MCP locked up after ~5 interactions
- whole-site-mvp-claims-coverage methodology note — Preview chrome wedged after one `window.location.assign('/')`

## Fix recipe

1. **Guard against recursive error reporting.** In `web/lib/error-reporting/init.ts:53` (verify line range), wrap the `reportBrowserError` invocation in a guard:
   ```ts
   let lastFailureAt = 0;
   const COOLDOWN_MS = 30_000;
   async function reportBrowserError(...) {
     if (Date.now() - lastFailureAt < COOLDOWN_MS) return;
     try {
       await fetch(...);
     } catch (err) {
       lastFailureAt = Date.now();
       // SWALLOW — do NOT re-throw. Re-throwing would refire window.error.
     }
   }
   ```
2. **Stabilise the useEffect** at the call-site so it doesn't re-attach the listener on every navigation. The dep array should be empty (`[]`) so the listener attaches once per session, not once per page mount. Verify the existing `[source]` dep is actually stable (audit suggests it may be re-evaluating).
3. **Never throw from the error-reporting fetch path.** Swallow + log to console once per cooldown.
4. **Playwright spec** at `web/playwright/error-reporting/no-runaway-loop.spec.ts`: navigate `/landing → /skills → /workers → /team → /approvals` in sequence; assert the network panel shows < 10 POSTs to `/api/v1/meta/error-reporting` total (not 5+ per mount).

## Acceptance criteria

- [ ] Driving the SPA through 5 sequential page navigations triggers ≤ 1 POST to `/api/v1/meta/error-reporting` (cooldown guard active).
- [ ] CORS failure on the error-reporting endpoint does NOT itself fire a new `window.error` event.
- [ ] The Claude Preview MCP completes a full 6-scene runbook walk without wedging.
- [ ] New Playwright spec passes cold.

## May-touch

- `web/lib/error-reporting/init.ts` (cooldown guard + swallow re-throw)
- `web/lib/error-reporting/reporter.ts` (if the fetch helper lives in a sibling)
- `web/playwright/error-reporting/no-runaway-loop.spec.ts` (NEW)

## Out of scope

- Gateway CORS allowlist widening (separate brief: `fix-gateway-cors-localhost-127-and-preview-proxy-origins`)
- Sentry / Datadog actual ingest pipeline wiring (not in v1.0 scope)
- Server-side error-reporting log retention policy

## Effort

~40-80 LOC + 1 Playwright spec. ~2-3 hours. P1 because it silently breaks every browser-driven audit + would block the demo MP4 if the recording runs > 5 minutes.
