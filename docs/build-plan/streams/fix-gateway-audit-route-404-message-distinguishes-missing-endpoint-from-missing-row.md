# fix-gateway-audit-route-404-message-distinguishes-missing-endpoint-from-missing-row

**Owner**: claude or codex (fast-mode candidate, ~30 LOC).
**Authored against**: origin/main `f208bcb70` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-audit-route-404-message-distinguishes-missing-endpoint-from-missing-row`.
**Source**: evaluator round 83 (2026-05-26T08:10Z) — informational ergonomic finding from live-gateway canonical-routes probe: `curl /api/v1/audit/feed?limit=3` returns 404 with body `"audit row not found"`. Misleading — the **route** doesn't exist at that path (the actual route is `/api/v1/audit` or `/api/v1/audit/events?limit=...`); the 404 should say "endpoint not found" rather than implying a row was searched for.

🟢 **P3 — operator/developer ergonomics fix.** Curling a non-existent audit subroute returns a 404 whose body text is the row-lookup error from a different code path. The Axum router falls through to a default-404 handler that returns the wrong message for "route not registered." Operators (and future evaluators) waste time diagnosing "missing row" when the real problem is "I curled the wrong path."

## Acceptance criteria

- [ ] Reproduce: `curl -sf http://127.0.0.1:7400/api/v1/audit/feed?limit=3 -o /tmp/audit-feed.json; cat /tmp/audit-feed.json` — confirm current body is "audit row not found" or similar row-language.
- [ ] Identify the fallback 404 site (likely the Axum router's `.fallback()` or a wildcard handler in `crates/argo-cli/src/handlers/gateway/router.rs` or similar). Cite the exact file:line.
- [ ] Make the fallback return a body like `{"error": "endpoint_not_found", "path": "/api/v1/audit/feed", "hint": "see /api/v1/audit or /api/v1/audit/events"}` for any `/api/v1/audit/...` path that doesn't match a registered route.
- [ ] Keep the row-lookup 404 (when the route IS registered but the requested row is missing) unchanged — that one is correct.
- [ ] Acceptance test: `curl -sf -m 2 :7400/api/v1/audit/feed?limit=3` returns 404 with body containing `"endpoint_not_found"` and NOT containing `"row"`. Existing `/api/v1/audit/sessions/<missing-uuid>` should still return its row-not-found body (regression check).

## May-touch

- `crates/argo-cli/src/handlers/gateway/router.rs` (or wherever the router fallback is wired)
- `crates/argo-cli/src/handlers/gateway/routes_audit*.rs` (only if the existing row-404 needs disambiguation)
- A new or extended integration test under `crates/argo-cli/tests/`

## Out of scope

- Adding the `/api/v1/audit/feed` route itself — not a real surface, just a typo-in-the-curl scenario.
- Refactoring the row-not-found error path — it's correct as-is.
- Broader API error-format standardisation — separate brief if desired.

## Worker note

Quick ergonomic fix. Falls under operator-locked `--skip-clippy` fix-* / ops-* / eval-* auth after evaluator pre-verification. No mission-claim impact; purely operator-quality-of-life.
