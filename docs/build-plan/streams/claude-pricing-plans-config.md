# claude-pricing-plans-config — Free/Pro/Enterprise tier config + feature gates (Wave 2 P1)

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-pricing-plans-config`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5). Without tier definitions, no commercial story.
- **Enterprise positioning** — explicit tiers signal product maturity.

## Scope

P1. Define 3 tiers + feature gate enforcement:

**Free:** 1 worker, 100 actions/month, mock provider only, community support.
**Pro ($29/mo):** 5 workers, 5,000 actions/month, real provider keys, email support, Slack/email/SMS channels, marketplace skills.
**Enterprise (custom):** unlimited workers, dedicated support, SSO, audit log export, custom verticals, voice channel, priority queueing.

Implementation:
- TOML config: `docs/skycot/pricing/v1-tiers.toml` — canonical source of tier definitions.
- Rust deserializer in new module `argo-billing::tiers` (NEW crate or `argo-identity::tier` extension — pick whichever is cleaner; default to extending `argo-identity`).
- `Tenant` row gains a `tier` column (free/pro/enterprise).
- Feature-gate enforcement: at action dispatch, check `tier.allow_action(action_kind, current_usage)`. Fail-closed with clear error.
- Usage metering: per-tenant action counter persisted to `tenant_usage` table; reset monthly.
- API: `GET /api/v1/tenant/usage` returns current usage + tier + remaining quota.

LOC: ~250.

## Files you may touch

- `docs/skycot/pricing/v1-tiers.toml` (NEW) — canonical pricing config. ≤80 LOC.
- `crates/argo-identity/src/tier.rs` (NEW) — `Tier` enum + `TierLimits` struct + `allow_action()` method. ≤140 LOC.
- `crates/argo-identity/src/lib.rs` — `pub mod tier;` + re-export. ≤10 modified lines.
- `crates/argo-store/migrations/<next>__tenant_tier.sql` (NEW) — additive: add `tier TEXT NOT NULL DEFAULT 'free'` to tenant table. ≤10 LOC.
- `crates/argo-store/migrations/<next>__tenant_usage.sql` (NEW) — additive: usage counter table. ≤20 LOC.
- `crates/argo-store/src/tenant.rs` — narrowly: extend with tier accessor. ≤30 modified lines.
- `crates/argo-cli/src/handlers/gateway/usage_endpoint.rs` (NEW) — `GET /api/v1/tenant/usage`. ≤80 LOC.
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly: mount route. ≤10 modified lines.
- Test sibling. ≤80 LOC.

## Files you must NOT touch

- `argo-billing` if it exists separately — out of scope for v1 (no separate billing crate yet).
- Stripe integration — separate brief (`claude-stripe-subscription-integration`).
- Existing tenant code — extend, don't refactor.

## Acceptance criteria

- [ ] `v1-tiers.toml` defines 3 tiers with quotas + feature flags.
- [ ] `Tenant::tier` field exists; defaults to `free`.
- [ ] `Tier::allow_action(action, usage)` returns `Allow`/`Deny(reason)`.
- [ ] Usage counter increments on action dispatch (test passing).
- [ ] `GET /api/v1/tenant/usage` returns correct shape: `{ tier, used, limit, resets_at }`.
- [ ] `cargo test -p argo-identity -p argo-cli tier` passes.
- [ ] `cargo clippy -p argo-identity -p argo-cli -p argo-store --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-pricing-plans-config`
- PR title: `feat(identity): pricing tiers + feature gates + usage metering (claude-pricing-plans-config)`

## Notes for the implementer

- Tier names lowercase: `free`, `pro`, `enterprise`. Match Stripe product names.
- Quota reset: monthly. Use `created_at` of the tenant as anchor for the reset date.
- `allow_action`: return enum, not bool, so callers can render user-friendly error.
- Usage metering writes can be best-effort batched if performance matters; not required for v1.
- Stripe subscription brief consumes `tier` field to swap on subscription change.
