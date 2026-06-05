# deploy/waf — Cloudflare WAF + edge rate limiting

Codifies the **edge** layer of the SKYCOT public-API abuse defense for `api.skycot.com`.
Standalone Terraform module (mirrors `deploy/dns/`). These rules sit **above** the gateway's
own per-IP / per-tenant / per-route limiter (`argo-config` `sections_rate_limit`), so abusive
anonymous traffic is shed at Cloudflare before it reaches the single Fargate task.

## What it provisions

- **`cloudflare_ruleset` (phase `http_ratelimit`)** — per-IP rate-limit rules:
  - `/api/v1/auth/*` → managed-challenge past `auth_requests_per_minute` (default 30/min/IP).
  - `/api/v1/genesis/*` → **block** past `genesis_requests_per_minute` (default 20/min/IP) —
    the strongest edge mitigation for runaway LLM cost from one IP.
  - `/api/v1/*` → block past `api_requests_per_minute` (default 600/min/IP) — generous global ceiling.
- **`cloudflare_ruleset` (phase `http_request_firewall_custom`)** — a baseline rule that
  managed-challenges auth requests with an empty `User-Agent` (a strong scripted-abuse signal).

Portable to the **Cloudflare Free plan** — rate-limit + custom-firewall phases only; no paid
managed-ruleset entitlement assumed.

## Apply (operator runs — NOT run by CI)

```sh
cd deploy/waf
export CLOUDFLARE_API_TOKEN=...   # Zone:WAF + Zone:Ruleset edit on skycot.com
terraform init
terraform plan  -var "zone_id=<zone-id>"
terraform apply -var "zone_id=<zone-id>"
```

Prereq: `api.skycot.com` is **proxied** through Cloudflare (orange cloud) — see
`deploy/dns/` and the AWS runbook §7. Rules only apply to proxied traffic.

## Validate locally (CI does this; no Cloudflare creds needed)

```sh
cd deploy/waf
terraform init -backend=false
terraform validate
terraform fmt -recursive -check
```

## Tuning

The per-IP caps are deliberately generous (catch abuse, not normal use). Tighten via the
variables in `variables.tf`. The dollar-denominated anon LLM-spend ceiling (a cumulative
$-cap, not a request-rate cap) is the **app-layer** follow-up
`sprint-ws5-4b-anon-llm-spend-ceiling-backend`; see
[`docs/skycot/deploy/abuse-and-cost-controls.md`](../../docs/skycot/deploy/abuse-and-cost-controls.md)
for the full layered model.
