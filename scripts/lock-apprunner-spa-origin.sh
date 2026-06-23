#!/usr/bin/env bash
# lock-apprunner-spa-origin.sh — lock the App Runner SPA origin (app.skycot.com /
# service skycot-prod-web) to Cloudflare-only, mirroring the ALB rung-A lock.
#
# WHAT IT DOES: creates an AWS WAFv2 REGIONAL web ACL with DefaultAction=Block + one
# Allow rule matching Cloudflare's published CIDRs (15 IPv4 + 7 IPv6), then associates
# it to the App Runner service. After this, the raw *.awsapprunner.com origin returns
# 403 to non-Cloudflare traffic, while app.skycot.com (via Cloudflare) keeps working.
#
# SAFE: no code change, no SPA redeploy, no new secret. App Runner health checks are
# TCP probes from inside the service (behind the WAF) → NOT affected. Rollback is one
# disassociate command (printed at the end), instant, zero downtime.
#
# NB: this is a PRODUCTION security change — run it yourself; the conductor prepared it.
set -euo pipefail
REGION=ap-southeast-2
APPRUNNER_ARN="arn:aws:apprunner:ap-southeast-2:123456789012:service/skycot-prod-web/00000000000000000000000000000000"

V4=(173.245.48.0/20 103.21.244.0/22 103.22.200.0/22 103.31.4.0/22 141.101.64.0/18 \
    108.162.192.0/18 190.93.240.0/20 188.114.96.0/20 197.234.240.0/22 198.41.128.0/17 \
    162.158.0.0/15 104.16.0.0/13 104.24.0.0/14 172.64.0.0/13 131.0.72.0/22)
V6=(2400:cb00::/32 2606:4700::/32 2803:f800::/32 2405:b500::/32 2405:8100::/32 \
    2a06:98c0::/29 2c0f:f248::/32)

echo ">> Step 1/4: create Cloudflare IPv4 + IPv6 IP sets (REGIONAL)…"
V4_ARN=$(aws wafv2 create-ip-set --name skycot-cloudflare-ipv4 --scope REGIONAL --region $REGION \
  --ip-address-version IPV4 --addresses "${V4[@]}" \
  --query 'Summary.ARN' --output text 2>/dev/null || \
  aws wafv2 list-ip-sets --scope REGIONAL --region $REGION \
    --query "IPSets[?Name=='skycot-cloudflare-ipv4'].ARN | [0]" --output text)
V6_ARN=$(aws wafv2 create-ip-set --name skycot-cloudflare-ipv6 --scope REGIONAL --region $REGION \
  --ip-address-version IPV6 --addresses "${V6[@]}" \
  --query 'Summary.ARN' --output text 2>/dev/null || \
  aws wafv2 list-ip-sets --scope REGIONAL --region $REGION \
    --query "IPSets[?Name=='skycot-cloudflare-ipv6'].ARN | [0]" --output text)
echo "   IPv4 IPSet: $V4_ARN"
echo "   IPv6 IPSet: $V6_ARN"

echo ">> Step 2/4: create the web ACL (DefaultAction=Block, Allow only Cloudflare)…"
RULES=$(cat <<JSON
[ {
  "Name": "AllowCloudflare", "Priority": 0,
  "Statement": { "OrStatement": { "Statements": [
    { "IPSetReferenceStatement": { "ARN": "$V4_ARN" } },
    { "IPSetReferenceStatement": { "ARN": "$V6_ARN" } }
  ] } },
  "Action": { "Allow": {} },
  "VisibilityConfig": { "SampledRequestsEnabled": true, "CloudWatchMetricsEnabled": true, "MetricName": "cf-allow" }
} ]
JSON
)
ACL_ARN=$(aws wafv2 create-web-acl --name skycot-prod-apprunner-waf --scope REGIONAL --region $REGION \
  --default-action Block={} --rules "$RULES" \
  --visibility-config SampledRequestsEnabled=true,CloudWatchMetricsEnabled=true,MetricName=skycot-prod-apprunner-waf \
  --query 'Summary.ARN' --output text 2>/dev/null || \
  aws wafv2 list-web-acls --scope REGIONAL --region $REGION \
    --query "WebACLs[?Name=='skycot-prod-apprunner-waf'].ARN | [0]" --output text)
echo "   web ACL: $ACL_ARN"

echo ">> Step 3/4: wait for WAF propagation (avoids WAFUnavailableEntityException)…"
sleep 20

echo ">> Step 4/4: associate the web ACL to the App Runner service…"
aws wafv2 associate-web-acl --web-acl-arn "$ACL_ARN" --resource-arn "$APPRUNNER_ARN" --region $REGION
echo "   ✓ associated."

cat <<EOF

DONE — the App Runner SPA origin is now Cloudflare-only (WAF default-Block + Allow-Cloudflare).
Tell Claude and it will verify (app.skycot.com via Cloudflare = 200; raw awsapprunner origin = 403; service still RUNNING).

ROLLBACK (instant, zero downtime, no redeploy):
  aws wafv2 disassociate-web-acl --resource-arn "$APPRUNNER_ARN" --scope REGIONAL --region $REGION
EOF
