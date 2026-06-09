# fix-genesis-intake-tenant-resolution

Authored against: origin/main c359fc5ca on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-genesis-intake-tenant-resolution

## Goal

Key the Genesis "Talk it through" intake store on the JWT-resolved tenant,
not a client-supplied header. Close the cross-tenant read/write/poison gap
on in-progress teammate drafts.

## Why now (security audit 2026-06-10, P1 — confirmed isolation gap)

- `crates/argo-cli/src/handlers/gateway/routes_genesis_intake_conversation.rs:74`
  uses `super::rate_limit::extract_tenant(req)` as the `IntakeStore` load/save
  key (`:105`, `:151`).
- `extract_tenant` (`rate_limit.rs:269-292`) honors `?tenant=` then
  `X-Tenant-Id`/`X-Argo-Tenant-Id` then config default — ALL client-controlled.
- Impact: an authenticated tenant-A user sends `X-Tenant-Id: <B>` and reads or
  poisons tenant B's in-flight teammate spec (which can later be hired). With
  no header, all users collapse onto the config-default tenant.
- The canonical resolver exists: `resolve_tenant::resolve_tenant_for_request`
  (`resolve_tenant.rs:60-73`, JWT `tnt` claim first — the WS1 keystone,
  pattern PRs #2710/#2754). #2737 fixed only the audit-WRITE tenant for this
  family, not the store key.
- Same class, check + fix in the same PR if in scope:
  `routes_voice_realtime.rs:207,245` (mitigated but not closed by #2754's
  SQL scope).

## Fix

Replace `extract_tenant(req)` with `resolve_tenant_for_request(req, store)`
for the intake-store key on every genesis-intake route in that file (and the
voice-realtime occurrences if they still key client-side). `extract_tenant`
remains acceptable ONLY for rate-limit bucketing of anonymous lanes — if
that's its remaining use, add a doc comment saying so and rename if cheap
(`extract_tenant_for_rate_bucket`) to prevent re-misuse.

## May touch

- `crates/argo-cli/src/handlers/gateway/routes_genesis_intake_conversation.rs`
- `crates/argo-cli/src/handlers/gateway/routes_voice_realtime.rs`
- `crates/argo-cli/src/handlers/gateway/rate_limit.rs` (doc comment / rename)
- their `_tests` siblings

## Acceptance / verifier

1. Cross-tenant negative test mirroring #2710's pattern: authed tenant-A
   request with `X-Tenant-Id: B` reads/writes ONLY tenant A's intake row;
   tenant B's store is untouched (assert store contents, not just status).
2. No-header authed request keys on the JWT tenant (not config default).
3. `cargo test -p argo-cli genesis_intake` + voice tests green.

## Size / owner

~40-100 LOC including tests. Owner: claude. 🔴 P1 SECURITY / tenant
isolation — DEC-034 adversarial-review surface; DEC-044 public-preview gate
item.
