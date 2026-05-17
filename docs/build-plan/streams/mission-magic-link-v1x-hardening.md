# mission-magic-link-v1x-hardening — clean-up the v1 magic-link shortcuts (v1.x P1)

**Owner**: claude. **Depends on**: `claude-rebase-pr-1756` MERGED (PR #1796 — when it lands).
**Authored against**: origin/main `de02c80a` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-magic-link-v1x-hardening`.
**Source**: PR #1796 follow-up — operator approved 3 sensible deviations for v1.0 ship; this brief addresses them in v1.x.

## Mission alignment

- **Repeatability** (CLAUDE.md priority #3) — canonical patterns across security primitives, not 1-off shortcuts.
- **Security** (#4) — proper signing-key derivation + real outbound email + canonical rate-limiter.
- **Commercial usability** (#5) — production-grade auth, not demo-grade.

## Scope

P1 (v1.x post-launch). Three cleanups from #1796's sensible-deviation list:

### 1. Move session cookie minting from SPA edge to gateway response

Current state (#1796): SPA's `/auth/verify` page reads the JWT from the verify response body and calls `document.cookie = '__Host-session=...'` via JS.

Target: `POST /api/v1/auth/verify` emits `Set-Cookie: __Host-session=...; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=...` directly. SPA never touches the cookie value.

Reasons:
- HttpOnly cookies can't be set from JS (current pattern violates the intent).
- Single source of truth for cookie attributes.
- Matches the `claude-jwt-cookie-security-hardening` brief's `__Host-` prefix discipline.

Change locations:
- Gateway `routes_auth.rs::verify` returns the cookie via the response builder, not in the JSON body.
- SPA `/auth/verify/page.tsx` drops the JS cookie-set; relies on the response header.

### 2. Migrate per-IP rate-limit from in-module to shared `rate_limit_ip.rs`

Current state (#1796): `auth_magic_link.rs` carries its own in-module HashMap-based per-IP token bucket with 5/hr default.

Target: use `gateway/rate_limit_ip.rs` (the shared limiter) with a new bucket scoped `signup` (5/hr). Same behavior, single rate-limit implementation.

Reasons:
- Single tunable knob in operator config (`[rate_limit_ip.signup]`).
- Reuses existing IPv6-/64 keying + observability metrics.
- Avoids two parallel rate-limit implementations the team has to maintain.

### 3. Real outbound email (SMTP or Postmark/SendGrid) replacing tracing-log fallback

Current state (#1796): `routes_auth.rs::signup` calls `tracing::info!("magic_link_url: {}", url)` instead of sending an email. Operator copy/pastes from logs for demo.

Target:
- Add `[email.outbound]` config block: `provider = "postmark" | "sendgrid" | "smtp" | "noop"`.
- `noop` (default) — current behavior (log only).
- `postmark` / `sendgrid` / `smtp` — real outbound. Use `argo-email-templates::MagicLinkEmail` (already MERGED #1766) for rendering.
- Operator flips to a real provider in v1.x via config edit; behavior preserved for v1.0 demos via `noop`.

LOC: ~400.

## Files you may touch

**Gateway (cookie + rate-limit migration):**
- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` (existing) — flip cookie-set into response; swap rate-limit call. ≤60 modified LOC.
- `crates/argo-cli/src/handlers/gateway/auth_magic_link.rs` (existing) — drop in-module rate-limit code; signup signature change to consume the shared limiter. ≤80 modified LOC.
- `crates/argo-cli/src/handlers/gateway/rate_limit_ip.rs` (existing) — add `signup` bucket variant. ≤30 modified LOC.

**SPA (cookie clean):**
- `web/app/auth/verify/page.tsx` — drop document.cookie line; rely on Set-Cookie header. ≤15 modified LOC.

**Outbound email:**
- `crates/argo-channels-email/src/outbound.rs` (NEW or existing) — provider abstraction + noop/postmark/sendgrid/smtp impls. ≤180 LOC.
- `crates/argo-config/src/sections_email_outbound.rs` (NEW) — `[email.outbound]` config schema. ≤50 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` — wire outbound::send_magic_link() in place of the tracing log. ≤20 modified LOC.
- Tests for each provider stub. ≤80 LOC.

## Files you must NOT touch

- The magic-link token table (`magic_link_tokens`) — schema unchanged.
- `argo-email-templates::MagicLinkEmail` — read-only consumer.
- JWT issuance logic — read-only.
- Tenant / user_account / user_tenant tables — read-only.

## Acceptance criteria

- [ ] `POST /api/v1/auth/verify` response includes `Set-Cookie: __Host-session=...; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=<exp>`.
- [ ] SPA `/auth/verify` no longer manipulates document.cookie.
- [ ] Signup endpoint consumes `rate_limit_ip::signup_bucket` instead of in-module limiter; `auth_magic_link.rs` no longer carries its own HashMap.
- [ ] `[email.outbound]` config block parses (default `provider = "noop"`).
- [ ] With `provider = "noop"`: tracing log preserved (no regression in v1.0 demo behavior).
- [ ] With `provider = "postmark"` + valid API key: outbound email sent via Postmark, MagicLinkEmail template rendered.
- [ ] With `provider = "smtp"` + valid creds: outbound email sent via SMTP.
- [ ] `cargo test -p argo-cli -p argo-channels-email -p argo-config` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` clean for touched crates.
- [ ] `pnpm typecheck && pnpm test` clean for web changes.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-magic-link-v1x-hardening`
- Title: `refactor(auth): server-side cookie + shared rate-limit + outbound email provider (mission-magic-link-v1x-hardening)`
- Worker verification: crate-scoped + workspace clippy + web checks per SWARM_PROTOCOL.md.

## Hidden constraints

- **Pre-claim check: confirm #1796 MERGED.** This brief is strictly a follow-up to land after the magic-link feature itself.
- **Cookie attributes must match `claude-jwt-cookie-security-hardening`** when that brief lands — `__Host-` prefix discipline. If both briefs are unmerged, this one defers to the other on cookie shape.
- **Don't break the v1.0 demo.** With `provider = "noop"` (default), tracing log output stays so operator can copy the magic-link from terminal. Only changes outbound when operator opts in.
- **Postmark/SendGrid creds in `Redacted<String>`** per the established pattern (#1774 MERGED).
- **No retry logic in this brief.** Send fails → log + return 200 (silent fail). Real retry is `codex-billing-webhook-resilience`-style follow-up if needed.
