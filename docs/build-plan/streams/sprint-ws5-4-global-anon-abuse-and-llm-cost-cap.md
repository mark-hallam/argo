# sprint-ws5-4-global-anon-abuse-and-llm-cost-cap

**Owner**: claude (Windows lane B — backend+infra lane).
**Depends on**: nothing for the infra/edge half. The backend dollar-spend ceiling is split to a paired follow-up (see below).
**Authored against**: origin/main `07ff2ddfc` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws5-4-global-anon-abuse-and-llm-cost-cap`.

## Why

`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS5-4): *"global per-IP /
per-anon-tenant LLM-spend ceiling + WAF/Cloudflare rules; verify Free tier truly stays on
mock provider."* The audit flags the global anonymous LLM-spend cap as **unverified**.

### What already exists (verified this turn, code-read)

- **Per-IP rate limiting — EXISTS + ON by default.** Connection-level `IpRateLimiter::from_config(&gateway_cfg.rate_limit.ip)` (`boot_serve.rs:379`), config `[gateway.rate_limit.ip]` (`sections_rate_limit.rs:40`), default `enabled=true`, capacity 120, refill 2/s. A dedicated tighter `signup_limiter()` also guards signup (`auth_magic_link_signup.rs:28`).
- **Per-tenant + per-route rate limiting — EXISTS** (config-driven: `[gateway.rate_limit.tenant]` + `[gateway.rate_limit.route]`, with overrides like `forge.run=10`, `skill.execute=30`).
- **Per-(authenticated)-tenant budget subsystem — EXISTS** (`turn_budget.rs`, `handlers/budget*.rs`, `costs*.rs`).

### The real gaps

1. **No hard global anon LLM-DOLLAR-spend ceiling.** The Genesis authoring path (`argo-genesis/src/author.rs`) has zero `budget`/`spend`/`cost`/`quota` references — it uses the operator's keys with no per-anon-IP dollar cap. The per-IP *request*-rate limiter bounds request volume (hence indirectly spend) but is not a dollar ceiling.
2. **Free→mock is SOFT, not hard.** The workflow execute path defaults `provider` to `"mock"` when unspecified (`routes_verticals_run.rs:56,60`), but the tier feature flag `provider_real_keys` is **never checked at runtime** (`rg provider_real_keys crates/` → only `v1-tiers.toml` + tests). So the "Free stays on mock" guarantee rests on the per-request default + the absence of a saved real key, NOT on a tier gate. The hard tier→capability gate is **WS4-3**'s job; this brief verifies + documents the current soft state honestly (NO THEATER).
3. **No edge WAF.** Cloudflare fronts the deploy (`deploy/dns/cloudflare-records.tf`, proxied=true) but no WAF / edge rate-limit rules are codified.

## What

Two halves; this brief ships the **infra/edge + verification** half (the Windows lane).
The Rust dollar-ceiling is a paired follow-up (can't compile-test argo-cli on the Windows
box — see memory `windows-cannot-build-argo-cli-cryptoki`).

### A. Cloudflare WAF + edge rate-limit rules as code (NEW `deploy/waf/`)
- `cloudflare_ruleset` (phase `http_ratelimit`): per-IP rules on the expensive/abusable
  paths — `/api/v1/auth/*` + `/api/v1/genesis/*` tighter, `/api/v1/*` a generous global
  ceiling — `action = managed_challenge`/`block`, characteristics `["ip.src"]`.
- `cloudflare_ruleset` (phase `http_request_firewall_custom`): a baseline custom rule
  (e.g. challenge obvious no-UA / known-bad automated traffic to `/api/v1/auth/*`).
- Portable to the Cloudflare Free plan (rate-limit + custom-firewall phases; no paid
  managed-ruleset entitlement assumed). Standalone module mirroring `deploy/dns/`
  (own provider block + `zone_id` var). Operator applies separately.

### B. Verification + cost-controls doc (NEW `docs/skycot/deploy/abuse-and-cost-controls.md`)
- Honestly verify Free→mock (soft today; cross-ref WS4-3 for the hard gate).
- Document the layered defense: Cloudflare edge → gateway IP/tenant/route limiter →
  per-tenant budget — with **recommended production values** for the existing config knobs.
- Specify the missing **hard global anon LLM-dollar ceiling** as a design + point at the
  paired backend brief.

### C. Paired backend follow-up (authored as a TODO brief, NOT implemented here)
- `sprint-ws5-4b-anon-llm-spend-ceiling-backend` (TODO) — a per-anon-IP / per-anon-tenant
  cumulative LLM-spend guard that fails closed (or forces mock) past a configurable
  ceiling, wired into the Genesis/voice/pitch LLM dispatch. Owner = Mac swarm (needs
  cargo compile + tests). Authored here so it enters the queue.

## May-touch
- `deploy/waf/**` (NEW — cloudflare WAF module).
- `docs/skycot/deploy/abuse-and-cost-controls.md` (NEW).
- `docs/build-plan/streams/sprint-ws5-4b-anon-llm-spend-ceiling-backend.md` (NEW — TODO brief).
- `docs/build-plan/BUILD_STATUS.md` (ledger flip on PR open + the 5-4b TODO row).

Do NOT change any Rust crate in this PR (the dollar-ceiling is the paired follow-up).

## Acceptance criteria
- [ ] `cd deploy/waf && terraform init -backend=false && terraform validate` exits 0.
- [ ] `terraform fmt -recursive -check deploy/waf` exits 0.
- [ ] WAF ruleset has per-IP rate-limit rules on `/api/v1/auth/*` + `/api/v1/genesis/*` + a global `/api/v1/*` ceiling, characteristics `["ip.src"]`.
- [ ] `abuse-and-cost-controls.md` verifies Free→mock honestly (soft, cross-ref WS4-3), documents the layered defense + recommended prod values, and specifies the dollar-ceiling design.
- [ ] The paired `sprint-ws5-4b-...` backend brief exists + has a BUILD_STATUS TODO row.
- [ ] No file > 500 LOC; no Rust file changed.

## Out of scope
- The Rust dollar-spend ceiling (paired follow-up `sprint-ws5-4b-...`).
- The hard tier→capability gate / Free real-key block (WS4-3).
- A live Cloudflare apply (operator-gated).

## Parallel-surface check
- `rg 'cloudflare_ruleset' deploy/` — none today; `deploy/dns/cloudflare-records.tf` only
  manages DNS records, not WAF. New, non-overlapping.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS5-4). Sprint playbook (WS5).
Backend/infra-lane Windows lane B mandate.
