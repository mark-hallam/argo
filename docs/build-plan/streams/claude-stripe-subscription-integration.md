# claude-stripe-subscription-integration — Stripe Checkout + webhook → tenant tier activation (Wave 2 P1)

**Owner**: claude. **Depends on**: `claude-pricing-plans-config` MERGED + `claude-tenant-provisioning-endpoint` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-stripe-subscription-integration`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5).
- **Self-serve readiness** — without billing, no monetization of viral traffic.

## Scope

P1. Stripe subscription lifecycle. **Test mode for v1 weekend**; production-mode approval submitted in parallel by operator.

Flow:
1. SPA route `/billing/subscribe?plan=pro` → POST `/api/v1/billing/checkout` → backend creates Stripe Checkout session → returns URL → SPA redirects.
2. User completes Checkout on Stripe.
3. Stripe → webhook to `POST /api/v1/billing/webhook` → backend verifies signature → finds `tenant_id` from Stripe metadata → updates `tenant.tier = 'pro'`.
4. Stripe subscription status changes (cancelled, past_due, etc.) → webhook → updates tier accordingly.
5. SPA route `/billing/portal` → POST `/api/v1/billing/portal` → backend creates Stripe Customer Portal session → returns URL → SPA redirects. Customer can manage subscription, payment methods, invoices.

Stripe products/prices: defined in `docs/skycot/pricing/v1-tiers.toml` → seeded into Stripe via one-time script.

Idempotency: webhook deduplication via `stripe_event_id` table.

LOC: ~1500.

## Files you may touch

**Backend:**
- `crates/argo-billing/Cargo.toml` (NEW) — new crate (Stripe API integration). ≤40 LOC.
- `crates/argo-billing/src/lib.rs` (NEW) — public API. ≤30 LOC.
- `crates/argo-billing/src/stripe_client.rs` (NEW) — Stripe HTTP client wrapper. ≤200 LOC.
- `crates/argo-billing/src/checkout.rs` (NEW) — checkout session creation. ≤120 LOC.
- `crates/argo-billing/src/webhook.rs` (NEW) — webhook handler + signature verification + idempotency. ≤250 LOC.
- `crates/argo-billing/src/portal.rs` (NEW) — customer portal session. ≤80 LOC.
- `crates/argo-billing/src/webhook_tests.rs` (NEW). ≤150 LOC.
- `crates/argo-store/migrations/<next>__stripe_customer.sql` (NEW) — `tenant_id` ↔ `stripe_customer_id` mapping. ≤20 LOC.
- `crates/argo-store/migrations/<next>__stripe_event.sql` (NEW) — idempotency dedup. ≤15 LOC.
- `crates/argo-cli/src/handlers/gateway/billing.rs` (NEW) — gateway-side handlers. ≤200 LOC.
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly: mount routes. ≤15 modified lines.
- `Cargo.toml` (workspace) — narrowly: add `argo-billing` member. ≤5 modified lines.

**SPA:**
- `web/app/(app)/billing/page.tsx` (NEW) — billing overview + upgrade CTA. ≤180 LOC.
- `web/app/(app)/billing/subscribe/page.tsx` (NEW) — landing for subscribe action. ≤80 LOC.
- `web/lib/billing/use-checkout.ts` (NEW). ≤60 LOC.
- `web/lib/billing/use-portal.ts` (NEW). ≤40 LOC.
- `web/__tests__/billing/*.test.tsx` (NEW). ≤120 LOC.

**Config:**
- `docs/skycot/pricing/stripe-seed.md` (NEW) — operator instructions for seeding Stripe products/prices. ≤80 LOC.

## Files you must NOT touch

- Existing `argo-mcp-connector-stripe-events` — that's for inbound payment-event capture; this is for subscription management.
- `argo-identity::tier` — consume only; tier updates flow via `Tenant::set_tier()`.
- Pricing config TOML — consume only.

## Acceptance criteria

- [ ] `POST /api/v1/billing/checkout` returns Stripe Checkout URL.
- [ ] Webhook verifies Stripe signature (HMAC + tolerance window).
- [ ] Webhook idempotency: same `stripe_event_id` processed twice = no-op.
- [ ] `customer.subscription.created` event → tenant.tier = 'pro' (or 'enterprise' for higher price ids).
- [ ] `customer.subscription.deleted` event → tenant.tier = 'free'.
- [ ] `customer.subscription.updated` event with `status=past_due` → tenant.tier = 'free' + warning email.
- [ ] `POST /api/v1/billing/portal` returns Stripe Customer Portal URL.
- [ ] `/billing` page in SPA shows current tier + manage button + plan upgrade options.
- [ ] Stripe test-mode integration verified end-to-end (manual op test ok).
- [ ] `cargo test -p argo-billing -p argo-cli` passes.
- [ ] `cargo clippy -p argo-billing -p argo-cli -p argo-store --all-targets -- -D warnings` clean.
- [ ] `pnpm lint && pnpm type-check && pnpm test` clean for web changes.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-stripe-subscription-integration`
- PR title: `feat(billing): Stripe checkout + webhook + customer portal (claude-stripe-subscription-integration)`

## Notes for the implementer

- Use `stripe-rust` crate (well-maintained official-ish Rust SDK).
- Webhook signature verification: use `stripe::Webhook::construct_event` (built-in).
- Stripe API key: `STRIPE_SECRET_KEY` env (test-mode `sk_test_...` for weekend).
- Webhook secret: `STRIPE_WEBHOOK_SECRET` env.
- Test-mode is FINE for the weekend launch. Production mode requires Stripe account verification — separate operator workstream.
- Customer portal: minimal config — let Stripe's default portal UI handle it.
- For Wave 5 fixes: if Stripe Rust SDK proves flaky, fall back to direct HTTPS calls via reqwest.
