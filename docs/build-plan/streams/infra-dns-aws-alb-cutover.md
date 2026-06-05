# infra-dns-aws-alb-cutover

**Owner**: claude (Windows lane B — backend+infra lane).
**Depends on**: nothing hard. Complements WS5-1 (#2683) — the ALB it points at.
**Authored against**: origin/main `54d3f5395` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh infra-dns-aws-alb-cutover`.

## Why

`deploy/dns/cloudflare-records.tf` points `api.skycot.com` + `try.skycot.com` at **Fly.io**
A/AAAA records (`var.fly_ipv4`/`var.fly_ipv6`). The sprint moves the gateway to **AWS ECS
Fargate behind an ALB** (WS5-1, `infra/terraform/`). An ALB has a DNS name, not a stable IP,
so `api`/`try` must become **CNAMEs → the ALB**. Today the DNS-as-code would point public
traffic at the wrong (Fly) origin.

## What

Make `deploy/dns/cloudflare-records.tf` AWS-aware, **non-breaking**:
- Add `alb_dns_name` variable (default `""`). When set, `api` + `try` become CNAMEs to the
  ALB. When empty, they fall back to the legacy Fly A/AAAA records (rollback path).
- Make `fly_ipv4`/`fly_ipv6` optional (`default = ""`) — only needed for the Fly fallback.
- The apex / `www` / `app` records stay on Vercel either way.
- Update `deploy/dns/README.md`: AWS path callout, Terraform Path C (AWS + Fly variants),
  the subdomain map (api/try → ALB CNAME), the SSL handoff (ALB serves the ACM cert), and
  the verification curl note.

## May-touch
- `deploy/dns/cloudflare-records.tf` (edit — add alb_dns_name + conditional api/try records).
- `deploy/dns/README.md` (edit — AWS path docs).
- `docs/build-plan/BUILD_STATUS.md` (ledger flip on PR open).

Do NOT touch `deploy/dns/setup.sh` (the bash path; a follow-up if the operator prefers it).

## Acceptance criteria
- [ ] `cd deploy/dns && terraform init -backend=false && terraform validate` exits 0.
- [ ] `terraform fmt -check deploy/dns` exits 0.
- [ ] With `alb_dns_name` set, the plan would create `api`/`try` as CNAMEs to the ALB (no Fly A/AAAA).
- [ ] With `alb_dns_name` empty + Fly IPs set, the plan reproduces the current Fly behavior (non-breaking).
- [ ] README documents both paths + the rollback.
- [ ] No Rust file changed.

## Out of scope
- A live Cloudflare apply (operator-gated).
- `setup.sh` (bash) AWS support.
- The ALB/ACM provisioning itself (WS5-1 #2683).

## Parallel-surface check
- `deploy/dns/cloudflare-records.tf` — the only DNS-as-code; edited in place, no parallel.
- `deploy/waf/` (WS5-4) — separate concern (WAF rules), same zone; no overlap.

## Source
WS5-1 (#2683) makes the gateway an AWS ALB; this points DNS at it. Audit
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS5 — "TLS termination, public bind
need an AWS story"). Deploy index `docs/skycot/deploy/README.md`.
