#!/usr/bin/env bash
# SKYCOT — apply Cloudflare DNS records for *.skycot.com via API.
# Idempotent: upserts records matched by (name, type).
#
# Required env vars:
#   CLOUDFLARE_API_TOKEN   API token with Zone:DNS edit on skycot.com
#   CLOUDFLARE_ZONE_ID     Zone ID (Cloudflare → Overview, right column)
#   FLY_IPV4 / FLY_IPV6    Dedicated Fly.io IPs (flyctl ips list -a <app>)
#
# See deploy/dns/README.md for the full runbook.

set -euo pipefail

: "${CLOUDFLARE_API_TOKEN:?required}"
: "${CLOUDFLARE_ZONE_ID:?required}"
: "${FLY_IPV4:?required}"
: "${FLY_IPV6:?required}"

TTL="${TTL:-300}"
API="https://api.cloudflare.com/client/v4"
AUTH="Authorization: Bearer ${CLOUDFLARE_API_TOKEN}"
JSON="Content-Type: application/json"

command -v jq >/dev/null || { echo "jq required" >&2; exit 1; }
command -v curl >/dev/null || { echo "curl required" >&2; exit 1; }

# Vercel apex IPs — vercel.com/docs/concepts/projects/domains.
VERCEL_A="76.76.21.21"
VERCEL_AAAA="2606:4700:90:0:f22e:fbec:5bed:a9b9"
VERCEL_CN="cname.vercel-dns.com"

# Record set: "name|type|content"; all proxied.
RECORDS=(
  "@|A|${VERCEL_A}"
  "@|AAAA|${VERCEL_AAAA}"
  "www|CNAME|${VERCEL_CN}"
  "app|CNAME|${VERCEL_CN}"
  "api|A|${FLY_IPV4}"
  "api|AAAA|${FLY_IPV6}"
  "try|A|${FLY_IPV4}"
  "try|AAAA|${FLY_IPV6}"
)

upsert() {
  local name="$1" type="$2" content="$3"
  local fqdn payload existing_id
  if [[ "$name" == "@" ]]; then fqdn="skycot.com"; else fqdn="${name}.skycot.com"; fi
  payload=$(jq -nc --arg n "$fqdn" --arg t "$type" --arg c "$content" --argjson ttl "$TTL" \
    '{name:$n, type:$t, content:$c, ttl:$ttl, proxied:true}')
  existing_id=$(curl -fsS -H "$AUTH" \
    "${API}/zones/${CLOUDFLARE_ZONE_ID}/dns_records?name=${fqdn}&type=${type}" \
    | jq -r '.result[0].id // empty')
  if [[ -n "$existing_id" ]]; then
    echo "  update ${type} ${fqdn} -> ${content}"
    curl -fsS -X PUT -H "$AUTH" -H "$JSON" -d "$payload" \
      "${API}/zones/${CLOUDFLARE_ZONE_ID}/dns_records/${existing_id}" >/dev/null
  else
    echo "  create ${type} ${fqdn} -> ${content}"
    curl -fsS -X POST -H "$AUTH" -H "$JSON" -d "$payload" \
      "${API}/zones/${CLOUDFLARE_ZONE_ID}/dns_records" >/dev/null
  fi
}

echo "Applying ${#RECORDS[@]} records to zone ${CLOUDFLARE_ZONE_ID} (TTL=${TTL}s, proxied=true)"
for r in "${RECORDS[@]}"; do
  IFS='|' read -r name type content <<<"$r"
  upsert "$name" "$type" "$content"
done
echo "Done. Verify with: dig +short app.skycot.com  &&  curl -sI https://app.skycot.com/"
