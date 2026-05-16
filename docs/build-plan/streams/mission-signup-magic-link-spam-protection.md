# mission-signup-magic-link-spam-protection — rate-limit + bot defense on `/api/v1/auth/signup` (P1 pre-launch)

**Owner**: codex. **Depends on**: `claude-magic-link-auth-backend` MERGED (currently DIRTY #1756 — codex can pre-claim once it lands).
**Authored against**: origin/main `111a6559` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-signup-magic-link-spam-protection`.
**Source**: pre-launch hardening — public marketing-site CTA pointing at signup is a magic-link spam target. Today's auth backend has no rate-limit/bot-protection. Before traffic, lock it down.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — own-goal: a single curl loop could send unlimited magic-link emails, exhaust Postmark/SendGrid quota, get domain reputation flagged.
- **Security** (#4) — bot account harvesting prevention.
- **Commercial usability** (#5) — Anthropic/Postmark/SendGrid bills.

## Scope

P1. Add three layers of defense to `POST /api/v1/auth/signup`:

### Layer 1: per-IP rate limit (5 per hour)

- Use existing `tenant_rate_limit` infrastructure but keyed by IP (since pre-auth has no tenant).
- Storage: in-memory LRU with 24-hour TTL (no DB persistence; restart resets — that's fine).
- Over-quota → `429 Too Many Requests` with `Retry-After` header.
- IPv6 keyed by `/64` prefix to prevent address-cycling.

### Layer 2: per-email cooldown (60 seconds between magic-link issuance for same email)

- A second magic-link request for `email=X` within 60s of the first → `429` with `Retry-After: <remaining>`.
- Prevents accidental double-clicks AND limits scope of email-enumeration attacks.

### Layer 3: optional Cloudflare Turnstile (operator-configurable)

- `[auth.signup_protection.turnstile]` config block. If `enabled = true` AND `secret_key` set, signup endpoint expects a `cf-turnstile-response` field in the POST body.
- Server verifies token via `https://challenges.cloudflare.com/turnstile/v0/siteverify`.
- Failed verification → `403`.
- If config block absent → behavior unchanged (Turnstile is opt-in).
- Frontend (out of scope here; file follow-up `web-signup-turnstile-widget` for SPA wiring).

### Honeypot field

- The signup form (frontend) ships with a hidden `<input name="honeypot_url">` styled `display:none`. Bots fill it; humans don't see it.
- Server: if `honeypot_url` field non-empty → silent `200 OK` with `{ status: "sent" }` (don't tip off the bot) AND no actual email send.
- Costs nothing, blocks ~80% of naive scrapers.

LOC: ~250.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` — add 4 checks before issuing magic link. ≤80 modified LOC.
- `crates/argo-cli/src/handlers/gateway/signup_protection.rs` (NEW) — IP/email rate-limit + Turnstile verify + honeypot. ≤140 LOC.
- `crates/argo-cli/src/handlers/gateway/signup_protection_tests.rs` (NEW) — per-IP / per-email / Turnstile-stub / honeypot. ≤100 LOC.
- `crates/argo-config/src/sections_auth.rs` (existing or NEW) — `[auth.signup_protection]` config schema. ≤40 modified LOC.
- `crates/argo-cli/Cargo.toml` — narrowly: ensure `reqwest` (already present). ≤2 modified LOC.

## Files you must NOT touch

- Magic-link issuance logic itself (`argo-identity::magic_link` or wherever it lives) — read-only consumer.
- SPA-side Turnstile widget — separate brief.
- Tenant-scoped rate limit infra — that's per-tenant; this is pre-auth (per-IP / per-email).

## Acceptance criteria

- [ ] 6th signup attempt from same IP within 1 hour → 429 with `Retry-After` header.
- [ ] 2nd signup attempt for same email within 60s → 429 with `Retry-After`.
- [ ] Honeypot field non-empty → 200 (silent) AND no audit row written for email-send, AND no actual email queued.
- [ ] With Turnstile enabled + valid token → 200 success.
- [ ] With Turnstile enabled + invalid token → 403.
- [ ] With Turnstile disabled (default) → no Turnstile check happens.
- [ ] IPv6 keyed by `/64` (test: `2001:db8::1` and `2001:db8::2` share the same bucket).
- [ ] Server restart drops in-memory rate-limit state (acceptable; lifetime documented).
- [ ] No regression: well-formed signup with no rate-limit hit still issues a magic link.
- [ ] `cargo test -p argo-cli -p argo-config` passes.
- [ ] `cargo clippy -p argo-cli -p argo-config --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-signup-magic-link-spam-protection`
- Title: `feat(auth): IP+email rate limit + honeypot + optional Turnstile on signup (mission-signup-magic-link-spam-protection)`
- Worker verification: crate-scoped clippy + tests + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Pre-claim check: confirm #1756 MERGED.** Worker `git log origin/main -- crates/argo-cli/src/handlers/gateway/routes_auth.rs` should show the magic-link signup handler exists.
- **IP detection** — use `X-Forwarded-For` first-hop OR connection peer IP. Document precedence (config-driven: `[auth.trust_proxy_header] = true|false`).
- **Email lowercased for keying** — `Mark@Example.COM` and `mark@example.com` share the cooldown bucket.
- **Don't email-enum-leak.** All 429 / 403 responses look the same regardless of whether the email is already in the system. Same 60s cooldown applies to fresh + existing emails.
- **Turnstile siteverify is a network call** with its own latency budget — 2s timeout, fail-closed (403 on Turnstile network error, NOT silent-pass).
- **The 5-per-hour-per-IP is conservative** — adjust per operator-config if too tight for shared NAT environments (corporate networks). Default stays at 5; `[auth.signup_protection.ip_limit_per_hour]` config knob.
- **Constant-time honeypot check.** Don't `if !honeypot.is_empty()`; do a constant-time string-compare to avoid timing leakage.
- **No logs of the honeypot value** — could carry data.
