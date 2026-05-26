# fix-gateway-error-envelope-standardize-structured

**Owner**: codex.
**Authored against**: origin/main `271a3d4cc2a90ad40949400340a0464a19f48d5f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-error-envelope-standardize-structured`.
**Source**: UX-AUDIT-T0119MEL `docs/skycot/ux-audits/2026-05-27-0119/scene-6-claim-6-trust-and-approvals.md` F10 (P2).

🟡 **P2 — Gateway error envelopes are inconsistent across endpoints. `/api/v1/audit/aaaa` returns `{"error":{"code":"not_found","message":"audit row not found"}}` (structured). `/api/v1/workflow-runs` returns `{"error":"not found"}` (string). `/api/v1/workers` returns `{"error":"not found"}` (string). The SPA's typed error path expects `error.code` — a string error breaks parsing and the SPA falls through to its generic "Something went wrong" path on otherwise informative 404s.**

## Root cause

Different gateway route handlers serialize errors with different shapes:
- The audit handlers (post-#2402) use the structured envelope `{ error: { code: "not_found", message: "audit row not found" } }`.
- The workflow-runs / workers / verticals / older handlers use the legacy string-error envelope `{ error: "not found" }`.

The SPA's typed error layer (likely `web/lib/api/client.ts` `parseError()` or equivalent) was updated to read the structured shape; legacy endpoints still emit strings, so the SPA logs "Cannot read property 'code' of undefined" or silently falls through to mock-offline. Either way, the operator does not get the useful 404 message back.

The fix is shape-standardization across every gateway error path — pick the structured `{ error: { code, message } }` and migrate all string-error responders to use it.

## Findings this brief closes

- `docs/skycot/ux-audits/2026-05-27-0119/scene-6-claim-6-trust-and-approvals.md` F10 [P2] — `/api/v1/workflow-runs` returns `{"error":"not found"}` (different error shape from rest of gateway)

## Fix recipe

1. Grep for every gateway responder that emits a JSON error: `rg '"error":\s*"' crates/argo-cli/src/handlers/gateway/`.
2. For each match, replace the string-error with the structured envelope. Use a shared helper if not already present:
   ```rust
   fn error_response(code: &str, message: &str, status: StatusCode) -> Response {
       (status, Json(json!({ "error": { "code": code, "message": message } }))).into_response()
   }
   ```
3. Update the SPA-side `parseError()` to be lenient (read `error.code` if structured, fall back to `error` if string, fall back to generic) so the shape transition doesn't blow up the older clients during the migration.
4. Add a contract test in `crates/argo-cli/tests/gateway_error_envelopes.rs` that asserts every 4xx / 5xx response carries `{ error: { code, message } }` (use the gateway's full route table).
5. Document the envelope contract in `docs/skycot/operator/migrations.md` or a new `gateway-error-envelopes.md`.

## Acceptance criteria

- [ ] Every gateway 4xx/5xx response across the full route table emits `{ error: { code: STR, message: STR } }`.
- [ ] Contract test in `crates/argo-cli/tests/gateway_error_envelopes.rs` exercises ≥20 routes and asserts the shape.
- [ ] SPA-side `parseError()` is lenient (accepts both shapes during the transition; logs a warning when string shape encountered).
- [ ] `curl -s 'http://127.0.0.1:7400/api/v1/workflow-runs?limit=2'` returns the structured shape.
- [ ] No regression to the 200 / 201 success paths.

## May-touch

- `crates/argo-cli/src/handlers/gateway/*.rs` (~10-20 files, ~5-30 LOC each)
- `crates/argo-cli/src/handlers/gateway/errors.rs` (NEW or extended — shared helper)
- `crates/argo-cli/tests/gateway_error_envelopes.rs` (NEW — ~100 LOC contract test)
- `web/lib/api/client.ts` (~10 LOC for lenient parsing)
- `docs/skycot/operator/migrations.md` (or new `gateway-error-envelopes.md` — ~40 LOC contract docs)

## Out of scope

- Audit-endpoint error envelopes (already structured; PR #2402 already shipped that).
- The deeper 404 semantics question (gateway vs tenant-scoped 404 vs role-scoped 403) — that's a separate security review.
- The CORS-fix for cross-origin requests (covered by `fix-spa-client-use-same-origin-gateway-rewrite`).

## Effort

M-L — ~400 LOC across handlers + helper + contract test + SPA lenient parser + docs. ~4-6 hours. P2 because the SPA mostly recovers gracefully via mock-offline fallback today, but the inconsistency is a footgun for any agent reading gateway responses (and for any operator debugging via curl).
