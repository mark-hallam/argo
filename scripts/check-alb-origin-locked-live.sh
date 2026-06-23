#!/usr/bin/env bash
# check-alb-origin-locked-live.sh — go-live hard-gate against LIVE AWS (not a plan).
#
# The sibling gate scripts/check-alb-origin-locked.sh inspects a Terraform PLAN
# JSON only ("no AWS calls"). That catches drift you are about to introduce, but
# it is blind to drift that already exists in the account — e.g. someone widened
# the security group or repointed the listener in the AWS console, or an
# out-of-band `aws` call ran. This script closes that gap by reading the LIVE
# prod resources with read-only describe-* calls and failing on three drift rungs:
#
#   RUNG A — security-group ingress on 80 or 443 contains 0.0.0.0/0 or ::/0
#            (origin open to the whole internet, bypassing Cloudflare WAF +
#            rate-limits + the LLM-cost block).
#   RUNG B — the 443 listener DEFAULT action is not the 403 fixed-response
#            (so an un-headered request would reach the app instead of 403).
#   RUNG C — no listener rule forwards on the secret origin-auth header
#            (X-Skycot-Origin-Auth), i.e. the Cloudflare-only path is gone.
#
# It is CI-safe: every AWS call is a read-only describe-*; it mutates nothing.
#
# Usage:
#   bash scripts/check-alb-origin-locked-live.sh
#   AWS_REGION=ap-southeast-2 PROD_ALB_SG=sg-0example000000000 \
#     ORIGIN_AUTH_HEADER=X-Skycot-Origin-Auth bash scripts/check-alb-origin-locked-live.sh
#
# Exit 0 = LIVE origin is locked (SG Cloudflare-only, 443 default=403, header rule present).
# Exit 1 = LIVE drift on one or more rungs (prints which rung + the offending value).
# Exit 2 = usage / environment error (no aws CLI, no creds, SG or ALB not found).
set -uo pipefail

# --- config (env-overridable so the same script serves any account/region) -----
REGION="${AWS_REGION:-${AWS_DEFAULT_REGION:-ap-southeast-2}}"
SG_ID="${PROD_ALB_SG:-sg-0example000000000}"
AUTH_HEADER="${ORIGIN_AUTH_HEADER:-X-Skycot-Origin-Auth}"
GUARDED_PORTS=(80 443)
OPEN_V4="0.0.0.0/0"
OPEN_V6="::/0"

case "${1:-}" in
  -h | --help)
    sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
esac

fail() { echo "FAIL: $*" >&2; }
err()  { echo "ERROR: $*" >&2; exit 2; }

command -v aws     >/dev/null 2>&1 || err "aws CLI not on PATH"
command -v python3 >/dev/null 2>&1 || err "python3 not on PATH (needed to parse describe JSON)"

# Fail fast (and distinctly from a drift FAIL) if creds are missing/expired, so a
# CI auth problem reads as exit 2 (env error) not exit 1 (drift).
aws sts get-caller-identity --region "$REGION" >/dev/null 2>&1 \
  || err "no usable AWS credentials for region $REGION (sts get-caller-identity failed)"

AWS=(aws --region "$REGION" --output json)

# ------------------------------------------------------------------- RUNG A ----
# Security-group ingress on 80/443 must NOT contain 0.0.0.0/0 or ::/0.
SG_JSON="$("${AWS[@]}" ec2 describe-security-groups --group-ids "$SG_ID" \
  --query 'SecurityGroups[0].IpPermissions' 2>/dev/null)" \
  || err "describe-security-groups failed for $SG_ID (does it exist in $REGION?)"
[ -n "$SG_JSON" ] && [ "$SG_JSON" != "null" ] || err "security group $SG_ID returned no ingress permissions"

RUNG_A_OPEN="$(
  SG_JSON="$SG_JSON" OPEN_V4="$OPEN_V4" OPEN_V6="$OPEN_V6" \
  GUARDED="${GUARDED_PORTS[*]}" python3 <<'PY'
import json, os
open_v4, open_v6 = os.environ["OPEN_V4"], os.environ["OPEN_V6"]
guarded = {int(p) for p in os.environ["GUARDED"].split()}
perms = json.loads(os.environ["SG_JSON"]) or []
offending = []
for p in perms:
    fp, tp = p.get("FromPort"), p.get("ToPort")
    if fp is None or tp is None:
        continue
    if not any(fp <= g <= tp for g in guarded):
        continue
    for r in p.get("IpRanges") or []:
        if r.get("CidrIp") == open_v4:
            offending.append(f"{fp}-{tp} ipv4 {open_v4}")
    for r in p.get("Ipv6Ranges") or []:
        if r.get("CidrIpv6") == open_v6:
            offending.append(f"{fp}-{tp} ipv6 {open_v6}")
print("\n".join(offending))
PY
)"
if [ -n "$RUNG_A_OPEN" ]; then
  fail "RUNG A — prod ALB security group $SG_ID is OPEN to the internet on a guarded port:"
  printf '  %s\n' $RUNG_A_OPEN >&2
fi

# ----------------------------------------------- discover the ALB from the SG --
ALB_ARN="$("${AWS[@]}" elbv2 describe-load-balancers \
  --query "LoadBalancers[?SecurityGroups[?@=='$SG_ID']].LoadBalancerArn | [0]" \
  --output text 2>/dev/null)"
[ -n "$ALB_ARN" ] && [ "$ALB_ARN" != "None" ] \
  || err "no load balancer in $REGION uses security group $SG_ID"

L443_ARN="$("${AWS[@]}" elbv2 describe-listeners --load-balancer-arn "$ALB_ARN" \
  --query "Listeners[?Port==\`443\`].ListenerArn | [0]" --output text 2>/dev/null)"
[ -n "$L443_ARN" ] && [ "$L443_ARN" != "None" ] \
  || err "no HTTPS:443 listener on ALB $ALB_ARN"

# ------------------------------------------------------------------- RUNG B ----
# 443 listener DEFAULT action must be the 403 fixed-response.
DEFAULT_DESC="$("${AWS[@]}" elbv2 describe-listeners --load-balancer-arn "$ALB_ARN" \
  --query "Listeners[?Port==\`443\`].DefaultActions[0].{Type:Type,Status:FixedResponseConfig.StatusCode} | [0]" 2>/dev/null)"
RUNG_B_BAD="$(
  DEFAULT_DESC="$DEFAULT_DESC" python3 <<'PY'
import json, os
d = json.loads(os.environ["DEFAULT_DESC"] or "null") or {}
if d.get("Type") != "fixed-response" or str(d.get("Status")) != "403":
    print(f"type={d.get('Type')!r} status={d.get('Status')!r}")
PY
)"
if [ -n "$RUNG_B_BAD" ]; then
  fail "RUNG B — 443 listener default action is NOT the 403 fixed-response: $RUNG_B_BAD"
fi

# ------------------------------------------------------------------- RUNG C ----
# At least one rule on the 443 listener must forward on the secret origin-auth
# header (Cloudflare stamps X-Skycot-Origin-Auth; a request without it gets 403).
HEADER_RULE="$("${AWS[@]}" elbv2 describe-rules --listener-arn "$L443_ARN" \
  --query "Rules[?Conditions[?Field=='http-header' && HttpHeaderConfig.HttpHeaderName=='$AUTH_HEADER'] && Actions[?Type=='forward']].Priority | [0]" \
  --output text 2>/dev/null)"
if [ -z "$HEADER_RULE" ] || [ "$HEADER_RULE" = "None" ]; then
  fail "RUNG C — no 443 listener rule forwards on the secret origin-auth header '$AUTH_HEADER' (Cloudflare-only path is gone)."
  RUNG_C_BAD=1
fi

# ------------------------------------------------------------------- verdict ---
if [ -n "$RUNG_A_OPEN" ] || [ -n "$RUNG_B_BAD" ] || [ -n "${RUNG_C_BAD:-}" ]; then
  echo "NO-GO: live AWS origin is NOT locked — see RUNG failures above." >&2
  echo "Remediate in Terraform (do NOT hand-edit the console) and re-run; see AWS_DEPLOY_RUNBOOK.md §9/§12." >&2
  exit 1
fi

echo "PASS: live prod origin is locked — SG $SG_ID Cloudflare-CIDR-only on 80/443,"
echo "      443 default=403 fixed-response, and rule pri=$HEADER_RULE forwards on '$AUTH_HEADER'."
exit 0
