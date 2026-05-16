# claude-email-template-system — branded HTML email templates (Wave 4 P1)

**Owner**: claude. **Depends on**: `claude-brand-identity-system` MERGED + `claude-magic-link-auth-backend` IN_REVIEW (#1756).
**Authored against**: origin/main `054d8b0a` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-email-template-system`.
**Source**: §4 weekend plan Wave 4 — magic-link emails currently plaintext; brand impression starts at the first email.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — first-touch email IS the brand impression.
- **Reliability** (#1) — well-formed templates pass spam filters (DKIM/SPF aren't enough — content matters).
- **Repeatability** (#3) — single template engine, no copy-paste between handlers.

## Scope

P1. Polished HTML email templates for 4 trigger surfaces, plus the template engine.

Triggers:

1. **Magic-link signup** — `MagicLinkEmail { magic_url, email, expires_in }`. Subject: "Sign in to SKYCOT". Body: warm welcome + big button + plaintext fallback URL + expiry note + footer with support email.
2. **Welcome onboarding** — `WelcomeEmail { user_name, workspace_name }`. Fires after first successful provision. Body: "Welcome to SKYCOT, $name" + 3-step "first 5 minutes" guide + links to docs + Discord invite.
3. **Payment receipt** — `ReceiptEmail { tenant_name, amount, period, invoice_url }`. Fires from Stripe `invoice.paid` webhook. Body: line-item summary + portal link + next-billing-date.
4. **Support reply** — `SupportReplyEmail { ticket_id, reply_body, reply_from }`. Fires from support-queue handler. Body: threaded reply + ticket link.

Template engine:
- **Single Rust crate `argo-email-templates`.**
- Built on `askama` (compile-time typed templates) or `tera` (runtime) — pick `askama` for type safety + zero-cost runtime.
- All templates extend a base layout that contains: header (logo + tagline), body slot, footer (legal + unsubscribe + contact).
- Inline CSS only (email-client compatibility — Gmail/Outlook strip `<style>` blocks selectively).
- Brand tokens hard-coded in base layout — no runtime token interpolation (avoids XSS surface).
- Mobile-responsive via percentage widths + `meta name="viewport"`.
- Plaintext fallback generated automatically per template.

LOC: ~400.

## Files you may touch

**New crate:**
- `crates/argo-email-templates/Cargo.toml` (NEW). ≤30 LOC.
- `crates/argo-email-templates/src/lib.rs` (NEW) — public API + render functions. ≤80 LOC.
- `crates/argo-email-templates/templates/base.html` (NEW) — shared layout. ≤120 LOC.
- `crates/argo-email-templates/templates/magic_link.html` (NEW). ≤80 LOC.
- `crates/argo-email-templates/templates/welcome.html` (NEW). ≤100 LOC.
- `crates/argo-email-templates/templates/receipt.html` (NEW). ≤120 LOC.
- `crates/argo-email-templates/templates/support_reply.html` (NEW). ≤80 LOC.
- `crates/argo-email-templates/src/plaintext.rs` (NEW) — HTML → plaintext converter. ≤60 LOC.
- `crates/argo-email-templates/src/render_tests.rs` (NEW). ≤150 LOC.

**Integration:**
- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` — replace plaintext magic-link with templated send. ≤20 modified LOC.
- `crates/argo-cli/src/handlers/gateway/routes_auth_logout.rs` (if exists per `claude-jwt-cookie-security-hardening`) — no change required.
- `crates/argo-billing/src/webhook.rs` (if `claude-stripe-subscription-integration` lands first) — wire receipt email. ≤15 modified LOC. **If the billing crate isn't yet merged, leave this integration as a TODO with explicit `// TODO(receipt-email): wire after argo-billing lands` marker.**
- `crates/argo-cli/src/handlers/gateway/routes_support_*.rs` — wire support-reply template. ≤15 modified LOC.
- `Cargo.toml` (workspace) — add `argo-email-templates` member. ≤5 modified LOC.

## Files you must NOT touch

- Outbound email transport (SMTP / Postmark / SendGrid plumbing) — consumes the template render output. Out of scope.
- Brand tokens themselves — read-only (paste current values into base.html).
- Stripe events crate — that's `argo-mcp-connector-stripe-events`, inbound only.

## Acceptance criteria

- [ ] `argo-email-templates` crate compiles standalone (`cargo check -p argo-email-templates`).
- [ ] `MagicLinkEmail::render() -> (subject, html, plaintext)` produces well-formed HTML email.
- [ ] All 4 template types tested (golden-file fixtures): magic_link, welcome, receipt, support_reply.
- [ ] Plaintext fallback is real plaintext (no HTML tags, line-wrapped at 72 chars).
- [ ] Templates render in dark mode (test against Gmail / Apple Mail dark-mode quirks via inline `prefers-color-scheme` media queries or solid background fallback).
- [ ] No external resources (images use inline SVG or skipped — `<img>` triggers tracking-pixel warnings in clients).
- [ ] Magic-link email passes [Mail-tester](https://www.mail-tester.com/) checks at ≥ 8/10 (operator verifies in a follow-up).
- [ ] Base layout footer includes: "© SKYCOT" + support@skycot.com + a one-click unsubscribe link (for non-essential emails; magic-link is essential so no unsubscribe needed but receipts/support need it).
- [ ] `cargo test -p argo-email-templates` passes.
- [ ] `cargo clippy -p argo-email-templates --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-email-template-system`
- Title: `feat(email): branded HTML email templates + plaintext fallback (claude-email-template-system)`
- Worker verification: crate-scoped tests + clippy + per SWARM_PROTOCOL.md `cargo clippy --workspace`.

## Hidden constraints

- **Inline CSS only.** `<style>` blocks get partially stripped by Gmail. Use a tool like Premailer (Rust crate available) at build-time, or hand-inline.
- **No web fonts.** Fall back to system stacks: `font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif`. Loading Inter from a CDN in email is a tracking-pixel vector + broken in many clients.
- **Tables for layout.** Modern HTML email is still 1998 — use `<table>` for layout, not flexbox/grid. Outlook 2007–2019 renders with Word's HTML engine.
- **Test surface matters.** Operator's CI for email isn't ready; the brief ships golden-file fixtures + manual rendering instructions. Adding Litmus / Email on Acid integration is a follow-up.
- **One-click unsubscribe.** RFC 8058 / CAN-SPAM — receipts + support emails must include a header `List-Unsubscribe: <https://...>, <mailto:unsubscribe@skycot.com>` AND a one-click `POST` variant. Magic-link is exempt (essential transactional).
