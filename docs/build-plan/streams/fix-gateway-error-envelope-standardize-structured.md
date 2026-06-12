# fix-gateway-error-envelope-standardize-structured

**Owner**: claude or codex (broadened from codex-only by conductor 2026-06-12).
**Depends on**: none.
**Authored against**: origin/main `271a3d4cc` on `2026-05-27`. **RE-SCOPED against `30b44956f` on `2026-06-12`** — conductor re-verified: the 2026-05-28 BLOCK ("touches every gateway route, scope too large") is OBSOLETE. The structured envelope spread incrementally; the flat-string remainder is **17 non-test `routes_*.rs` files** (canonical enumerator: `rg -l '"error":\s*"' crates/argo-cli/src/handlers/gateway/routes_*.rs | grep -v test`), including 4 bare-return literals (`routes_post_writes.rs:62`, `routes_post_sessions.rs:45+212` — the latter string-interpolates UNESCAPED `{e}` into JSON, a correctness bug on its own — and `routes_catalog.rs:325` which returns literal `# error` text).
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-error-envelope-standardize-structured`.
**Source**: UX-AUDIT-T0119MEL F10 (P2) — the SPA's typed error path expects `error.code`; flat-string errors fall through to the generic "Something went wrong".

## Fix

For every non-test hit of the enumerator grep: replace the flat
`{"error":"<msg>"}` shape with the structured
`{"error":{"code":"<code>","message":"<msg>"}}` using the established
helpers (`error_envelope` / `bad_request` / `not_found` / `server_error` —
see `routes_worker_capability_grant.rs` or `routes_workspace_settings.rs:391`
for the pattern; reuse, do NOT mint a new helper). Special cases:

- `routes_post_sessions.rs:45/:212`: must switch `format!` → `json!` (the
  current code breaks on a `"` in the error message — fix is also an
  injection-shape bug fix).
- `routes_catalog.rs:325`: check the route's content-type FIRST — if it
  serves markdown/plain text, keep a text error (make it honest:
  `# error: <reason>`) and document the exception in the PR body.
- Code values: reuse the codes the structured routes already use
  (`bad_request` / `not_found` / `internal_error` / domain-specific) —
  match nearest-sibling semantics, don't invent a new taxonomy.

Tests: extend ONE existing test per touched route family asserting
`error.code` is present on the error path (no new test files unless a
route has none).

## May touch

- the non-test files the enumerator grep names (+ their existing test files)
- if the diff would exceed ~400 LOC or 500-LOC-ceiling pressure appears on
  any file, STOP at a clean subset and open the remainder as a named
  follow-up in the PR body (no half-converted files).

## Acceptance / verifier

1. Enumerator grep (non-test) → 0 hits, or only documented
   content-type exceptions listed in the PR body.
2. Touched route tests green; `cargo clippy -p argo-cli` targeted clean.
3. One SPA-side sanity: `web/lib/api` typed error parsing path unchanged
   (no SPA edits needed — the structured shape is what it already expects).

~60–150 LOC across ~17 files. **🟡 P2 — closes UX-AUDIT F10; SPA typed
errors work on every route; fixes an unescaped-JSON bug en passant.**
