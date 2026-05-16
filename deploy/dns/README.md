# SKYCOT — DNS + SSL setup for `*.skycot.com` (Wave 2 P1)

Operator runbook for the 6 SKYCOT subdomains, fronted by Cloudflare
(proxy mode = orange-cloud) for DDoS protection, analytics, and
automatic HTTPS via Cloudflare Universal SSL.

Two equivalent paths: Cloudflare UI, or `setup.sh` /
`cloudflare-records.tf` for reproducible apply. Pick one; do not mix.

## Pre-flight

1. `skycot.com` is registered and nameservers point at Cloudflare:
   `dig NS skycot.com +short` returns two `*.ns.cloudflare.com`
   entries. If not, run "Add a Site" in Cloudflare first, update
   nameservers at the registrar, wait for propagation (< 24h).
2. `codex-hosted-deployment-flyio` is deployed: `flyctl ips list -a
   <app>` reports a dedicated IPv4 + IPv6. Capture both.
3. `codex-spa-vercel-deployment` is deployed: the SPA project is live
   on Vercel.

## Subdomain map

| Host                 | Type  | Target                       | Proxy | TTL    | Notes                           |
|----------------------|-------|------------------------------|-------|--------|---------------------------------|
| `skycot.com` (apex)  | A     | `76.76.21.21`                | ✅    | 5 min  | Vercel apex marketing site     |
| `skycot.com` (apex)  | AAAA  | `2606:4700:90:0:f22e:fbec:5bed:a9b9` | ✅ | 5 min | Vercel apex IPv6           |
| `www.skycot.com`     | CNAME | `cname.vercel-dns.com`       | ✅    | 5 min  | Redirects to apex via Vercel    |
| `app.skycot.com`     | CNAME | `cname.vercel-dns.com`       | ✅    | 5 min  | SPA (Vercel project)            |
| `api.skycot.com`     | A     | `<fly-ipv4>`                 | ✅    | 5 min  | Gateway (Fly.io dedicated IPv4) |
| `api.skycot.com`     | AAAA  | `<fly-ipv6>`                 | ✅    | 5 min  | Gateway (Fly.io dedicated IPv6) |
| `try.skycot.com`     | A     | `<fly-ipv4>`                 | ✅    | 5 min  | Demo workspace (same Fly app)   |
| `try.skycot.com`     | AAAA  | `<fly-ipv6>`                 | ✅    | 5 min  | Demo workspace IPv6             |
| `docs.skycot.com`    | CNAME | `cname.vercel-dns.com`       | ✅    | 5 min  | Optional — docs site            |
| `status.skycot.com`  | CNAME | `<provider-CNAME>`           | ❌    | 1 hr   | Optional — Statuspage etc.      |

Raise TTLs from 5 min → 1 hr after records verify. The `status.` host
is unproxied so the uptime provider serves its own TLS and Cloudflare
outages don't mask status reporting.

## Path A — Cloudflare UI

DNS → Records → Add record. Repeat for each table row, keeping
"Proxy status" enabled (orange cloud) where shown. Use A/AAAA at the
apex (CNAMEs aren't valid per RFC 1035; Cloudflare flattening adds
latency vs the A/AAAA path).

## Path B — `setup.sh` (Cloudflare API)

```bash
export CLOUDFLARE_API_TOKEN=...            # Edit zone DNS scope, skycot.com only
export CLOUDFLARE_ZONE_ID=...              # Overview pane, right column
export FLY_IPV4=...                        # flyctl ips list -a <app>
export FLY_IPV6=...
bash deploy/dns/setup.sh                   # Idempotent — re-runs are safe
```

The script `PUT`s each record (create-or-update) so partial failures
can be resumed without duplicate entries. Source `setup.sh` to see
exactly which records get created.

## Path C — Terraform

```bash
cd deploy/dns
export CLOUDFLARE_API_TOKEN=...
terraform init
terraform plan -var "zone_id=$CLOUDFLARE_ZONE_ID" \
               -var "fly_ipv4=$FLY_IPV4" -var "fly_ipv6=$FLY_IPV6"
terraform apply ...
```

Terraform state lives wherever the operator wires the backend; the
file ships with a `local` backend by default. Switch to S3/GCS for
shared operator access before adding teammates.

## Vercel + Fly handoff

After the DNS records resolve, add custom domains in each provider so
they will issue + serve the right TLS cert:

- **Vercel**: project Settings → Domains → add `skycot.com`,
  `www.skycot.com`, `app.skycot.com` (and `docs.` if enabled). Vercel
  verifies via the records above and provisions Let's Encrypt.
- **Fly.io**: `flyctl certs add api.skycot.com -a <app>` and
  `flyctl certs add try.skycot.com -a <app>`. Fly provisions Let's
  Encrypt + serves the cert at the origin; Cloudflare's proxy uses
  Full (Strict) mode against it.

In Cloudflare → SSL/TLS → Overview, set encryption mode to **Full
(Strict)**. This is required so the proxy validates the origin's
real cert instead of accepting any presented cert.

## Verification

```bash
# DNS resolves through Cloudflare's edge (104.x.x.x answers = proxied):
for host in skycot.com www.skycot.com app.skycot.com api.skycot.com try.skycot.com; do
  echo "== $host =="; dig +short "$host" A; dig +short "$host" AAAA
done

# TLS chain valid + cert subject matches host:
for host in skycot.com app.skycot.com api.skycot.com try.skycot.com; do
  echo "== $host =="
  echo | openssl s_client -servername "$host" -connect "$host":443 2>/dev/null \
    | openssl x509 -noout -subject -issuer -dates
done

# End-to-end origin reachability:
curl -sI https://app.skycot.com/             | head -1   # 200 OK (Vercel)
curl -sI https://api.skycot.com/health       | head -1   # 200 OK (Fly gateway)
```

Once all checks pass, raise the TTL on each record to 3600 (1h) so
edge caching is friendlier. Keep 300 (5 min) on `api.` if you
anticipate a near-term Fly region/IP change.

## Rollback

Cloudflare → DNS → Records → delete the row, or `terraform destroy
-target=cloudflare_record.<name>`. Cached resolvers honor TTL.
