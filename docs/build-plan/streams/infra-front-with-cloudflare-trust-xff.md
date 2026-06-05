# infra-front-with-cloudflare-trust-xff

**Owner**: claude (Windows lane B — backend+infra lane).
**Depends on**: WS5-1 (#2683) + WS5-3 (#2685) — **STACKED on the WS5-3 branch** (edits the alb + ecs modules they created; rebases clean as the stack merges).
**Authored against**: origin/main `54d3f5395` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh infra-front-with-cloudflare-trust-xff`.

## Why

WS5-4 (`docs/skycot/deploy/abuse-and-cost-controls.md` §2) flagged: behind Cloudflare, the
gateway's per-IP rate limiter sees **Cloudflare's edge IP**, not the real client — so the
per-IP limits (and WS5-4's whole premise) are useless unless the gateway trusts
`X-Forwarded-For`. But blindly trusting XFF is spoofable if the origin is directly reachable.
The correct posture is **both**: (a) the ALB only accepts Cloudflare traffic, and (b) the
gateway trusts XFF.

The config IS env-settable (no `argo.toml` mount needed): `ARGO_TRUST_XFF`, `ARGO_IP_RATE_*`,
`ARGO_RATE_LIMIT_*` (`crates/argo-config/src/loader.rs:203-249`). So this is pure IaC wiring.

## What

One coherent `front_with_cloudflare` toggle (default `false` — non-breaking):
- **alb module:** when true, the ALB SG accepts 80/443 **only from Cloudflare's published IP
  ranges** (`cloudflare_ipv4_cidrs` / `cloudflare_ipv6_cidrs` vars, defaulted to the canonical
  list); when false, open to the internet (current behavior). IPv6 ingress added.
- **ecs module:** when `trust_xff` (= `front_with_cloudflare`), inject `ARGO_TRUST_XFF=1` +
  `ARGO_IP_RATE_LIMIT=1` so the per-IP limiter keys on the real client IP.
- **root:** `front_with_cloudflare` var threaded to both modules; tfvars example + runbook
  §7a (flip it on after DNS is on Cloudflare; keep false during bring-up smoke).

## May-touch
- `infra/terraform/modules/alb/{main.tf,variables.tf,README.md}` (SG + Cloudflare CIDRs).
- `infra/terraform/modules/ecs/{main.tf,variables.tf}` (ARGO_TRUST_XFF env).
- `infra/terraform/{main.tf,variables.tf,terraform.tfvars.example}` (wire the toggle).
- `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md` (§7a).
- `docs/build-plan/BUILD_STATUS.md` (ledger flip).

Do NOT change any Rust crate (the env vars already exist in argo-config).

## Acceptance criteria
- [ ] `cd infra/terraform && terraform validate` + `terraform fmt -recursive -check` exit 0.
- [ ] `front_with_cloudflare = false` (default) reproduces the current open-SG / no-XFF behavior (non-breaking).
- [ ] `front_with_cloudflare = true` → ALB SG ingress = Cloudflare CIDRs only; ECS task env has `ARGO_TRUST_XFF=1`.
- [ ] Runbook §7a documents the flip + the force-new-deployment to pick up the env.
- [ ] No file >500 LOC; no Rust changed.

## Out of scope
- A live apply (operator-gated).
- The per-route RPM tuning (the WS5-4 edge WAF already hard-caps genesis; app defaults are sane).
- Auto-refreshing the Cloudflare CIDR list (it's stable + published; a future cron/data-source could pull it).

## Parallel-surface check
- `rg 'front_with_cloudflare|ARGO_TRUST_XFF' infra/` — none before this; new.
- Reuses the existing `ARGO_TRUST_XFF` env handled by `argo-config` loader — no parallel config surface.

## Source
WS5-4 verification (`docs/skycot/deploy/abuse-and-cost-controls.md` §2 proxy note). Env vars:
`crates/argo-config/src/loader.rs`.
