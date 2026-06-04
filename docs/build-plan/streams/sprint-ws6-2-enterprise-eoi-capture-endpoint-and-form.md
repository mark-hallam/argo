# sprint-ws6-2-enterprise-eoi-capture-endpoint-and-form

**Owner**: claude or codex.
**Depends on**: none (independent). Pattern: mirror the waitlist endpoint shipped in #2675 (`routes_waitlist.rs`).
**Authored against**: origin/main `d699a56c8` on `2026-06-04` (verified: waitlist precedent @ `crates/argo-cli/src/handlers/gateway/routes_waitlist.rs`; enterprise mailto CTAs @ `web/app/pricing/page.tsx` + `web/app/contact/page.tsx`).
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws6-2-enterprise-eoi-capture-endpoint-and-form`.

## Why

The operator's sprint goal explicitly includes the enterprise track — "companies on EOI, getting
MOU, contracts." Today the enterprise interest path is a `mailto:` only (on `/pricing` +
`/contact`), so there is NO structured capture, no record, no audit trail, no follow-up data.
This brief builds the real enterprise Expression-of-Interest capture so inbound enterprise
leads are persisted + auditable — the front door for the EOI→MOU→contract motion.

## What

A public, anonymous-safe `POST /api/v1/enterprise-inquiry` that persists a structured
inquiry, mirroring the just-shipped waitlist endpoint (`routes_waitlist.rs`, #2675 — read it
first as the pattern).

- **Handler:** new `crates/argo-cli/src/handlers/gateway/routes_enterprise_inquiry.rs`.
  Parse `{ company (required), name (required), email (required), seats?, message? }`.
  Validate email shape (reuse `looks_like_email`). Reuse the existing signup honeypot + IP
  rate-limit (`rate_limit_ip::signup_limiter()`). Mount in the SAME public pre-bearer-gate lane
  as `/api/v1/waitlist` (in `routes.rs`, beside `auth_magic_link::dispatch` — NOT behind the
  bearer gate). Write an audit row (hash-chained — the moat applies to leads too).
- **Persistence:** additive migration for `enterprise_inquiry` (`id`, `company`, `name`,
  `email`, `seats`, `message`, `created_at`). Next free migration VERSION + SQL slot — re-verify
  both at claim time; bump `EXPECTED_SCHEMA_VERSION` + report it. Platform-level (pre-tenant),
  no `tenant_id` (same as waitlist). Idempotent-friendly (dedupe on email+company or just append
  — append is fine for leads; document the choice).
- **Frontend:** replace the `mailto:` enterprise CTA on `/pricing` (+ `/contact`, `/for/sales`)
  with a small inquiry form that POSTs the shape above (mirror `web/lib/api/waitlist.ts` +
  `WaitlistForm.tsx`). Honest success/error (NO THEATER — surface a real failure, don't fake).

## May-touch
- `crates/argo-cli/src/handlers/gateway/routes_enterprise_inquiry.rs` (NEW) + the public dispatcher arm + `mod.rs`
- migration `vNN.rs` (+ slot file if the pattern uses one) + `registry_compliance.rs` + `EXPECTED_SCHEMA_VERSION`
- `web/lib/api/enterprise-inquiry.ts` (NEW) + a form component + the 3 pages' CTAs
- `docs/build-plan/BUILD_STATUS.md` (ledger flip)

## Acceptance criteria
- [ ] `curl -X POST .../api/v1/enterprise-inquiry -d '{"company":"Acme","name":"A","email":"a@b.com"}'` → 200/202, inserts one row, writes an audit row.
- [ ] Malformed/missing required field → 400, no row.
- [ ] Honeypot/rate-limit reject path covered by a test.
- [ ] Frontend: the enterprise CTA opens the form (no more `mailto:`); submit shows real success; failure shows a real error.
- [ ] Migration slot check passes; `EXPECTED_SCHEMA_VERSION` bumped + reported.
- [ ] `cargo clippy -p argo-cli -p argo-store` clean; `pnpm typecheck` + vitest GREEN on touched web; report counts.
- [ ] No file >500 LOC, no fn >80 LOC, no `unwrap`/`expect` outside `#[cfg(test)]`.

## Out of scope
- CRM / email-provider forwarding of the lead (note as the natural next step; this brief persists + audits only).

## Parallel-surface check
- `rg 'routes_enterprise_inquiry|enterprise_inquiry' crates/` MUST be 0 at claim time. Mirror `routes_waitlist.rs`; do not fork a parallel public-form pattern.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS6-2). Pattern: `routes_waitlist.rs` (#2675).
