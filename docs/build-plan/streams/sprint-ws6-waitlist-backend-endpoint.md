# sprint-ws6-waitlist-backend-endpoint

**Owner**: claude or codex.
**Depends on**: none. **Independent quick-win — the landing page's waitlist form already exists and currently 404s.**
**Authored against**: origin/main `35d9225d2` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws6-waitlist-backend-endpoint`.

## Why

Day-0 gap-audit (WS6): the landing page's waitlist FORM is built and honest
(`web/components/marketing/WaitlistForm.tsx` + `web/lib/api/waitlist.ts`; per
NO-THEATER it surfaces the real failure rather than faking success), but
**`POST /api/v1/waitlist` does not exist** — confirmed by a live probe
(`{"error":"not found"}`) and by the absence of any `routes_waitlist*.rs`
handler. The frontend already documents the exact contract it expects
(`waitlist.ts:1-30`). This is the funnel's front door for organic traction — it
must capture real signups.

## What

Add a public, anonymous-safe `POST /api/v1/waitlist` that persists an email (+
optional name/company/source) to a new `waitlist_signups` table, deduped on
email, returning an idempotent `202 {"queued": true}`.

- **Handler**: new `crates/argo-cli/src/handlers/gateway/routes_waitlist.rs`.
  Parse `{ email (required), name?, company?, source? }`. Validate email shape.
  No auth required (public endpoint — it sits in the same un-gated lane as
  signup/verify; confirm the mount path by reading how `auth/signup` is mounted
  un-gated). Dispatch arm beside the other public POST routes (re-verify the
  exact dispatcher file at claim time — likely `routes_dispatch_post.rs` /
  `routes_dispatch_post_paths.rs`).
- **Bot/abuse guard**: reuse the existing signup protection
  (`signup_protection.rs` — honeypot + IP rate-limit via `rate_limit_ip.rs`).
  Do NOT invent a new limiter; wire the existing one. Public traffic-facing.
- **Persistence**: additive migration for `waitlist_signups`
  (`id`, `email` UNIQUE, `name`, `company`, `source`, `created_at`). Next free
  migration VERSION + SQL filename slot — re-verify both at claim time
  (`registry_compliance.rs` for the version, `scripts/check-migration-slots.sh`
  for the filename slot; bump `EXPECTED_SCHEMA_VERSION` in
  `crates/argo-store/src/lib.rs` and report the line in the PR body). Additive
  only — no table rewrite.
- **Dedupe**: `INSERT ... ON CONFLICT(email) DO NOTHING` (or pre-check) so a
  repeat signup is idempotent `202`, never a 500.
- **Multi-tenant note**: the waitlist is a **platform-level** (not per-tenant)
  surface — it's pre-signup. It does NOT need `tenant_id`. Confirm this is
  consistent with how signup (also pre-tenant) persists.

## May-touch
- `crates/argo-cli/src/handlers/gateway/routes_waitlist.rs` (NEW)
- the gateway POST dispatcher (1 arm) + `mod.rs` (`mod routes_waitlist;`)
- `crates/argo-store/src/migrations/v<NN>.rs` (NEW) + `migrations/<slot>__waitlist_signups.sql` (NEW) + `registry_compliance.rs` + `EXPECTED_SCHEMA_VERSION`
- `docs/build-plan/BUILD_STATUS.md` (ledger flip)

Do NOT touch the frontend `waitlist.ts`/`WaitlistForm.tsx` — they already POST
the correct shape (verify the shape matches; if it doesn't, align the handler to
the frontend, not vice-versa).

## Acceptance criteria
- [ ] `curl -X POST http://127.0.0.1:7400/api/v1/waitlist -d '{"email":"a@b.com"}'`
      returns `202 {"queued":true}` and inserts one row.
- [ ] Repeat POST with the same email returns `202` (idempotent), still one row.
- [ ] Malformed email → `400`, no row.
- [ ] Integration test in `crates/argo-cli/tests/` pinning insert + dedupe + the
      rate-limit/honeypot reject path.
- [ ] `scripts/check-migration-slots.sh` passes; `EXPECTED_SCHEMA_VERSION` bumped
      and reported in PR body.
- [ ] `cargo clippy -p argo-cli -p argo-store` clean; touched-crate tests GREEN.
- [ ] No file >500 LOC, no fn >80 LOC, no `unwrap`/`expect` outside `#[cfg(test)]`.

## Out of scope
- The enterprise EOI capture endpoint → **WS6-2** (separate brief).
- Any email-provider forward / confirmation email → follow-up; this brief only
  persists + dedupes. (Note it in the PR body as the natural next step.)

## Parallel-surface check
- `rg 'routes_waitlist|waitlist_signups' crates/` MUST return 0 at claim time.
- `rg 'fn .*waitlist' crates/` — confirm no in-flight handler.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS6). Frontend contract:
`web/lib/api/waitlist.ts:1-30`.
