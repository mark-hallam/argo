# sprint-ws2-owner-sees-unredacted-own-tenant-audit

**Owner**: claude.
**Authored against**: origin/main `eddcab780` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws2-owner-sees-unredacted-own-tenant-audit`.

## Why (VERIFIED by reading current main 2026-06-09)
A workspace OWNER viewing the audit feed of their OWN tenant sees their own
actions PII-redacted ("ping [EMAIL] to confirm" instead of the real address).
The hash-chained audit is the hero differentiator; an owner whose own actions
are hidden from them on their own tenant looks like the product hides data
from the buyer who paid for it. Cross-tenant redaction (the owner peeking at
another tenant via `as_tenant=`) is correct and MUST stay redacted.

Re-confirmed each file:line by reading it:
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:114` —
  `let policy = RedactionPolicy::default();` is built UNCONDITIONALLY for every
  request inside `list_response` (line 83), then passed to `audit_row_json`
  (line 135) and `redact_text` (line 279) for every row. The policy never
  consults the caller's role or whether the rows belong to the caller's own
  tenant — so an owner reading their own tenant gets `PolicyMode::Mask` on
  email/phone/card/iban/ird/address exactly like a guest.
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_query_tests.rs:284`
  `redacts_email_in_verb_by_default` — calls `get("/api/v1/audit?worker=w1")`
  (a request with NO `X-Argo-Role` header) and asserts the verb is masked to
  `[EMAIL]`. Under the role-gate dev-default (`effective_role`,
  `role_gate.rs:307`) a header-less in-crate-test request resolves to `Owner`,
  so this is the owner-on-own-tenant path — and today it redacts. This test
  encodes the WRONG behavior for the own-tenant-owner case and must be
  re-pointed (see Acceptance).
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_scope.rs:24`
  `resolve_tenant_scope` + `:48` `enforce_role(req, Action::AuditCrossTenant)`
  — cross-tenant access is already a first-class capability (owner-only, or a
  per-principal grant). `params.as_tenant` being `Some(other)` with the cap is
  the cross-tenant lane. This is the existing, correct gate we key off: own
  tenant = `as_tenant` absent (or equal to the caller's workspace tenant);
  cross-tenant = `as_tenant` names a different tenant.
- `crates/argo-cli/src/handlers/gateway/role_gate.rs:42` `enum Action`,
  `:146` `required_role`, `:196` `role_meets`, `:266` `resolve_role`,
  `:297` `enforce_role` — `Action::AuditFeedRead` floor is guest (line 73-76
  comment), `Action::AuditCrossTenant` floor is owner (line 79-82).
  `effective_role` is private (`fn` at `:307`); `resolve_role`, `role_meets`,
  `required_role` are `pub(super)`.
- `crates/argo-redact/src/policy.rs:51` `RedactionPolicy::default()` (all
  `Mask`), `:82` `RedactionPolicy::show_all()` (all `Show`, doc'd "Used by
  elevated `unredacted-view` flows") — `show_all()` is the existing
  consumer-ready constructor for the unredacted case. No new redact type
  needed.

## Grep-before-write
- `rg -n 'RedactionPolicy::default\(\)|RedactionPolicy::show_all|let policy' crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs`
  — confirm `:114` is the only policy construction in the feed handler.
- `rg -n 'redact_text|audit_row_json' crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs`
  — confirm `audit_row_json` (`:270`) is the sole projector that applies the
  policy; the detail render path (`routes_audit_detail_render.rs`) is OUT of
  scope for this brief (feed only).
- `rg -n 'pub\(super\) fn (resolve_role|role_meets|required_role)|enum Action|AuditCrossTenant' crates/argo-cli/src/handlers/gateway/role_gate.rs`
  — confirm the role helpers this brief reuses are `pub(super)`-visible to the
  feed module.
- New symbol grep (helper added by this brief):
  `rg 'pub (struct|enum|fn) feed_redaction_policy' crates/` — MUST be empty
  before write (no parallel surface). The new helper is a private
  module-level `fn feed_redaction_policy(req: &str, scope: &TenantScope) -> RedactionPolicy`
  in `routes_audit_feed.rs` (or `routes_audit_feed_scope.rs`); it is NOT a new
  pub type.

## Fix (additive, behavior-preserving except the owner-own-tenant case)
1. In `routes_audit_feed_scope.rs`, expose whether the resolved scope is the
   caller's OWN tenant (not the cross-tenant override lane). Add a
   `pub(super) fn is_own_tenant(scope: &TenantScope) -> bool` (or thread a
   bool out of `resolve_tenant_scope`): own-tenant = `!include_all_tenants`
   AND the `effective` tenant equals the request's workspace tenant (i.e.
   `params.as_tenant` did not redirect to a different tenant). The legacy-admin
   lane (`include_all_tenants`) and any `as_tenant=<other>` request are NOT own
   tenant.
2. In `routes_audit_feed.rs`, replace the unconditional
   `let policy = RedactionPolicy::default();` (`:114`) with a call to a new
   private helper `fn feed_redaction_policy(req: &str, scope: &TenantScope) -> RedactionPolicy`:
   - If the caller's `resolve_role(req)` (role_gate) is `Owner` AND
     `is_own_tenant(scope)` → `RedactionPolicy::show_all()` (owner sees their
     own tenant's data unredacted).
   - Otherwise → `RedactionPolicy::default()` (non-owner on own tenant, OR any
     cross-tenant / legacy-admin view, stays redacted — unchanged).
   Consumer: `list_response` at `routes_audit_feed.rs:114`; it already holds
   `req` and `scope` (built at `:99`), so the call is a one-line swap.
3. Do NOT touch the cross-tenant gate (`resolve_tenant_scope` /
   `Action::AuditCrossTenant`), the detail-render path, or `redact_text`
   itself. Status/scope semantics unchanged; only the masking policy for the
   owner-own-tenant case flips from `default()` to `show_all()`.
4. Keep each touched file < 500 LOC. `routes_audit_feed.rs` is the larger file
   — if the helper would push it over budget, place
   `feed_redaction_policy` + `is_own_tenant` in `routes_audit_feed_scope.rs`
   (which is ~94 LOC, ample headroom) and call across.

## Acceptance (truth-bounded)
1. Re-point the existing test: `routes_audit_feed_query_tests.rs:284`
   `redacts_email_in_verb_by_default` currently uses a header-less (=owner,
   own-tenant) request and asserts masking — that is now the UNREDACTED case.
   Rename/replace it so the masking assertion runs under a NON-owner request
   (e.g. via `get_with_tenant(path, tenant)` which stamps `X-Argo-Role: user`,
   per `routes_audit_feed_test_helpers.rs:138`), keeping the
   `contains("[EMAIL]")` + `!contains("sam@acmeco.com")` assertions for the
   non-owner.
2. Add a test `owner_sees_unredacted_own_tenant`: seed a row whose verb
   contains `sam@acmeco.com`, query as OWNER on the OWN tenant
   (`get_with_tenant_and_cap` stamps `X-Argo-Role: owner`,
   `routes_audit_feed_test_helpers.rs:148`, against the row's own tenant),
   assert the returned verb CONTAINS `sam@acmeco.com` and does NOT contain
   `[EMAIL]`.
3. Add a test `owner_cross_tenant_still_redacted`: owner with the cross-tenant
   cap querying `as_tenant=<other>` (the cross-tenant lane) — assert the verb
   is STILL masked (`[EMAIL]`, no raw address). Assert BOTH the
   own-tenant-unredacted and cross-tenant-redacted outcomes in the suite so
   the boundary is pinned in both directions.
4. Verifier: `cargo test -p argo-cli routes_audit_feed` green (the three tests
   above pass cold); `cargo check -p argo-cli` exit 0. Touched files each
   `wc -l` < 500.

## On ship (same commit)
- Update the `FEATURE_MATRIX.md` audit-feed / redaction row: note owner sees
  own-tenant audit unredacted; cross-tenant + non-owner redaction unchanged.
  (Do NOT regenerate `docs/skycot/feature-matrix.html` — it is gitignored per
  DEC-043; edit the `.md` row only.)
- One-line `docs/skycot/agents/SHARED.md` handoff noting the
  owner-own-tenant-unredaction fix.

## Effort
S–M (one policy-selection helper + an own-tenant predicate + 3 tests;
mechanical reuse of existing `show_all()` / `resolve_role` / `TenantScope`,
no new pub types, no schema change).
