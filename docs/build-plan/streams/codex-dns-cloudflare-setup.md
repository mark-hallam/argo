# codex-dns-cloudflare-setup — DNS records + SSL for *.skycot.com (Wave 2 P1)

**Owner**: codex. **Depends on**: `codex-hosted-deployment-flyio` MERGED + `codex-spa-vercel-deployment` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-dns-cloudflare-setup`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Commercial usability** — self-serve domain story.

## Scope

P1. Document DNS records + SSL for the 4 SKYCOT subdomains:
- `skycot.com` → Vercel (marketing site)
- `app.skycot.com` → Vercel (SPA)
- `api.skycot.com` → Fly.io (gateway)
- `try.skycot.com` → Fly.io or separate Vercel project (demo workspace)
- `docs.skycot.com` → Vercel (docs, optional)
- `status.skycot.com` → external (UpDown / Better Uptime / Statuspage)

Cloudflare proxy on all subdomains for DDoS protection + analytics + automatic HTTPS.

This brief delivers ONLY documentation + Terraform/scripts; operator executes the DNS changes manually in Cloudflare UI (or runs the script with their API token).

LOC: ~150.

## Files you may touch

- `deploy/dns/README.md` (NEW) — DNS setup runbook. ≤120 LOC.
- `deploy/dns/cloudflare-records.tf` (NEW, optional) — Terraform DNS config for reproducibility. ≤80 LOC.
- `deploy/dns/setup.sh` (NEW) — bash script using Cloudflare API + operator's API token. ≤80 LOC.

## Files you must NOT touch

- Any executable infrastructure code — these are configs only.
- Source code.

## Acceptance criteria

- [ ] `deploy/dns/README.md` documents all 6 subdomains + record types + targets + TTLs.
- [ ] DNS verification: after operator applies, `dig` returns expected records.
- [ ] SSL: Cloudflare Universal SSL + origin-pull cert on Fly.io.
- [ ] All subdomains use Cloudflare proxy (orange-cloud).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-dns-cloudflare-setup`
- PR title: `infra: DNS + SSL setup for *.skycot.com subdomains (codex-dns-cloudflare-setup)`

## Notes for the implementer

- Operator likely has skycot.com registered + on Cloudflare already. Verify in README first step.
- Use CNAME records pointing to vercel-dns.com (for Vercel) or fly.dev (for Fly.io). Both providers issue Letsencrypt certs automatically.
- Test record propagation: 5-min TTLs initially, raise to 1hr after verified.
- Status subdomain is optional in v1 if there's no external uptime monitor yet.
