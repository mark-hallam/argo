# SKYCOT — abuse + LLM-cost controls (public perimeter)

**Truth-bounded.** Every "exists" claim below cites the code read this turn. The Cloudflare
WAF is authored IaC (`deploy/waf/`), not yet applied. The hard dollar-spend ceiling is
**not built** — it's specified here + tracked as `sprint-ws5-4b-anon-llm-spend-ceiling-backend`.

This is the WS5-4 deliverable: what protects the public deploy from anonymous abuse +
runaway LLM cost, what's real today, and what's still missing.

## The layered model

```
            ┌──────────────────────────────────────────────────────────┐
  internet  │  1. Cloudflare edge (deploy/waf/)                         │
   ───────▶ │     per-IP rate limits + managed-challenge + bot signal   │   ← authored, not applied
            ├──────────────────────────────────────────────────────────┤
            │  2. Gateway per-IP limiter (IpRateLimiter)                │   ← LIVE, on by default
            │     + dedicated signup IP limiter                          │
            ├──────────────────────────────────────────────────────────┤
            │  3. Gateway per-tenant + per-route limiter                │   ← LIVE, config-driven
            ├──────────────────────────────────────────────────────────┤
            │  4. Per-(authenticated)-tenant budget + action quota      │   ← LIVE
            ├──────────────────────────────────────────────────────────┤
            │  5. Hard global anon LLM-DOLLAR ceiling                    │   ← MISSING (5-4b)
            └──────────────────────────────────────────────────────────┘
```

## 1. Cloudflare edge — `deploy/waf/` (authored this turn)

Per-IP rate-limit rules + a custom-firewall challenge, applied at Cloudflare before traffic
reaches the single Fargate task. Genesis (`/api/v1/genesis/*`, the expensive anon LLM path)
is **hard-blocked** past 20 req/min/IP; auth is managed-challenged past 30/min/IP; a generous
600/min/IP global ceiling backstops `/api/v1/*`. Portable to the Cloudflare Free plan. See
[`deploy/waf/README.md`](../../../deploy/waf/README.md). Operator applies it (Cloudflare creds).

## 2. Gateway per-IP limiter — LIVE (verified)

- Connection-level `IpRateLimiter::from_config(&gateway_cfg.rate_limit.ip)`
  (`crates/argo-cli/src/handlers/gateway/boot_serve.rs:379`), applied to every connection via
  `ConnectionCtx`.
- Config `[gateway.rate_limit.ip]` (`crates/argo-config/src/sections_rate_limit.rs:40`):
  `enabled = true` **by default**, `capacity = 120`, `refill_per_sec = 2.0`. So even with no
  `argo.toml`, every source IP is bounded to ~120 burst / 2 sustained per second.
- A dedicated, tighter `signup_limiter()` additionally guards signup
  (`auth_magic_link_signup.rs:28`), on top of Turnstile + honeypot (`signup_protection.rs`).
- **Proxy note:** behind Cloudflare, set `[gateway.behind_proxy]` (trusted peers) so the
  limiter keys on the real client IP, not Cloudflare's. `trust_xff` defaults to `false`.

## 3. Per-tenant + per-route limiter — LIVE (config-driven)

- `[gateway.rate_limit.tenant]` (default per-tenant RPM, burst multiplier; per-tenant
  overrides in the `tenant_rate_limit` table take precedence).
- `[gateway.rate_limit.route]` (`default_rpm = 100`, overrides `forge.run = 10`,
  `skill.execute = 30`). Tighten the expensive routes here.

### Recommended production `argo.toml` (mount or bake into the image)

The deploy injects env, not an `argo.toml`. The defaults above are sane and already active,
but to tighten the expensive routes, mount this:

```toml
[gateway.rate_limit.ip]
enabled        = true
capacity       = 120
refill_per_sec = 2.0

[gateway.rate_limit.tenant]
requests_per_minute = 120

[gateway.rate_limit.route]
default_rpm = 100
# Lower the LLM-heavy routes once their stable route-keys are confirmed
# (see argo-cli rate_limit route-key mapping). Example shape:
# [gateway.rate_limit.route.overrides]
# "forge.run"      = 10
# "skill.execute"  = 30
```

## 4. Per-tenant budget — LIVE

A full budget/spend subsystem exists for **authenticated** tenants (`turn_budget.rs`,
`handlers/budget*.rs`, `costs*.rs`) + the per-month action quota (`GET /api/v1/tenant/usage`,
`max_actions_per_month`). This bounds an authenticated tenant's spend; it does **not** bound
anonymous pre-auth traffic.

## 5. Free tier → mock provider — SOFT today (honest finding)

**Verified:** the workflow execute path defaults `provider`/`model` to `"mock"` when the
request omits them (`routes_verticals_run.rs:56,60`, `.unwrap_or("mock")`). **But** the tier
feature flag `provider_real_keys` (defined in `docs/skycot/pricing/v1-tiers.toml`) is **never
checked at runtime** — `rg provider_real_keys crates/` returns only the TOML + tests, nothing
on the dispatch path.

So "Free stays on mock" rests on (a) the per-request default being mock and (b) a Free user
not having a saved real provider key — **not** on a hard tier gate. A Free user who saves a
real key and explicitly passes `provider: "anthropic"` is **not** blocked by tier logic today.

**The hard gate is WS4-3** (`sprint-ws4-enforce-tier-capability-gates`): make Free actually
block real provider keys / >1 worker / paid channels per `v1-tiers.toml`. This doc verifies
the current soft state; WS4-3 closes it. NO THEATER: do not claim Free is hard-locked to mock.

## 6. MISSING — hard global anon LLM-dollar ceiling (→ `sprint-ws5-4b`)

The Genesis authoring path (`crates/argo-genesis/src/author.rs`) has **zero**
budget/spend/cost/quota references — it calls the operator's keys with no per-anon-IP dollar
cap. Voice/pitch are similar. Today the only bound on anon LLM spend is the **request-rate**
limit (layers 1–2), which caps volume, not dollars.

**`sprint-ws5-4b-anon-llm-spend-ceiling-backend`** (paired backend brief, Mac lane) specifies
a cumulative per-anon-IP / per-anon-tenant LLM-spend guard that fails closed (or forces the
mock provider) past a configurable ceiling, wired into the Genesis/voice/pitch dispatch. It's
split out because it needs Rust compile + tests, which the Windows infra lane can't run
(see memory `windows-cannot-build-argo-cli-cryptoki`).

## Operator checklist for the public launch

- [ ] Apply `deploy/waf/` (Cloudflare WAF) once `api.skycot.com` is proxied.
- [ ] Set `[gateway.behind_proxy]` trusted peers so the IP limiter sees real client IPs.
- [ ] (Optional) mount the tightened `argo.toml` rate-limit block above.
- [ ] Land WS4-3 to make Free→mock a hard gate.
- [ ] Land `sprint-ws5-4b` for the dollar-denominated anon spend ceiling.
