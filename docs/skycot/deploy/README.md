# SKYCOT — deploy overview (operator map)

One entry point for taking SKYCOT public. The platform is **two deployables** behind one
domain:

- **Gateway** (`api.skycot.com`) — the single `argo` binary on **AWS ECS Fargate**.
- **SPA** (`app.skycot.com`) — the `web/` Next.js app on **Vercel**.

DNS + the edge (TLS, WAF, rate limits) are **Cloudflare**.

```
  app.skycot.com ──► Vercel (Next.js SPA)
        │  XHR
        ▼
  api.skycot.com ──► Cloudflare (DNS + WAF + TLS) ──► AWS ALB ──► Fargate task (argo)
                                                                      │
                                                                      ▼
                                                                EFS /data (SQLite)
```

## The surfaces (what lives where)

| Surface | Path | Status |
|---|---|---|
| Gateway AWS IaC (Fargate, ALB, ACM, EFS, IAM, VPC) | [`infra/terraform/`](../../../infra/terraform/) | WS5-1 ([#2683](https://github.com/mark-hallam/Argo-Agent/pull/2683)) |
| Gateway deploy runbook (operator command sequence) | [`AWS_DEPLOY_RUNBOOK.md`](AWS_DEPLOY_RUNBOOK.md) | WS5-1 ([#2683](https://github.com/mark-hallam/Argo-Agent/pull/2683)) |
| Secrets (KMS CMK + Secrets Manager) | `infra/terraform/modules/{kms,secrets}` | WS5-3 ([#2685](https://github.com/mark-hallam/Argo-Agent/pull/2685)) |
| Canonical secret list (every prod env var) | [`deploy/secrets/skycot-production.env.example`](../../../deploy/secrets/skycot-production.env.example) | WS5-3 ([#2685](https://github.com/mark-hallam/Argo-Agent/pull/2685)) |
| SQLCipher image build (prod boot prereq) | root `Dockerfile` | [#2686](https://github.com/mark-hallam/Argo-Agent/pull/2686) |
| SPA Vercel runbook | [`deploy/vercel/README.md`](../../../deploy/vercel/README.md) | WS5-2 |
| DNS (Cloudflare records) | [`deploy/dns/`](../../../deploy/dns/) | shipped |
| WAF + edge rate limits | [`deploy/waf/`](../../../deploy/waf/) | WS5-4 ([#2688](https://github.com/mark-hallam/Argo-Agent/pull/2688)) |
| Abuse + LLM-cost controls (layered model) | [`abuse-and-cost-controls.md`](abuse-and-cost-controls.md) | WS5-4 ([#2688](https://github.com/mark-hallam/Argo-Agent/pull/2688)) |

> Status reflects the open PRs as of authoring; links resolve as each merges. **NO THEATER:**
> nothing here has been deployed to AWS/Vercel/Cloudflare — every `terraform apply` /
> Vercel import / DNS change is the operator's step.

## Operator order of operations

1. **Gateway image** — build the `argo` image **with `--features bundled-sqlcipher`** (or it
   won't boot in production — [#2686](https://github.com/mark-hallam/Argo-Agent/pull/2686)),
   push to ECR. Runbook §4.
2. **Gateway infra** — `terraform apply` in `infra/terraform/` (VPC, EFS, ALB, ECS, secrets,
   CMK). Runbook §1–3.
3. **Secrets** — `aws secretsmanager put-secret-value` for each entry (bearer, master key,
   JWT, **DB key**, Stripe). Runbook §6; full list in `deploy/secrets/`.
4. **ACM cert** — validate via Cloudflare CNAME (or Route53 if `route53_zone_id` set). Runbook §5.
5. **DNS** — point `api.skycot.com` (CNAME → ALB, proxied) via `deploy/dns/`. Runbook §7.
6. **WAF** — `terraform apply` in `deploy/waf/` once `api.skycot.com` is proxied. Set
   `[gateway.behind_proxy]` so the gateway's IP limiter sees real client IPs.
7. **SPA** — import `web/` to Vercel, set the env table + `app.skycot.com`. `deploy/vercel/README.md`.
8. **Smoke** — `curl https://api.skycot.com/health/ready` (200), `/api/v1/team` (401 = auth on),
   load `app.skycot.com`, cold signup → hire → approve.

## Known follow-ups (tracked)

- **WS4-2** (operator) — create the Stripe products matching `v1-tiers.toml` slugs; put the
  `price_…` ids in the gateway secret + the SPA `NEXT_PUBLIC_STRIPE_PRICE_*`.
- **WS4-3** — make Free→mock a **hard** tier gate (today it's soft; see
  `abuse-and-cost-controls.md` §5).
- **`sprint-ws5-4b`** — hard global anon LLM-dollar ceiling (edge caps bound volume, not dollars).
- **`infra-dockerfile-bundled-sqlcipher-feature`** — once merged, the image builds with the
  feature by default and runbook §4's manual step goes away.
