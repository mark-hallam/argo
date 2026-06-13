#!/usr/bin/env bash
# check-alb-origin-locked.sh — pre-deploy hard-gate for the AWS ALB origin.
#
# Refuses go-live when the prod ALB security-group ingress on 80/443 resolves to
# 0.0.0.0/0 or ::/0 (an internet-open origin bypasses Cloudflare WAF + rate-limits
# + the LLM-cost block). Operates purely on a Terraform plan JSON — no AWS calls,
# no network — so it is safe in CI and as the runbook §12 go/no-go gate.
#
# Produce the plan JSON with:
#   cd infra/terraform && terraform plan -out tfplan && terraform show -json tfplan > plan.json
# Then run either form:
#   bash scripts/check-alb-origin-locked.sh plan.json
#   terraform -chdir=infra/terraform show -json tfplan | bash scripts/check-alb-origin-locked.sh
#
# Exit 0 = ALB origin is Cloudflare-CIDR-only (or the plan's env is not prod).
# Exit 1 = OPEN origin on a prod plan (prints the offending ingress rule).
# Exit 2 = usage / parse error (missing file, unparseable JSON, no ALB SG found).
#
# Unlike the in-Terraform precondition, this gate does NOT honor the bring-up
# escape hatch (allow_open_origin_bringup): it inspects the RESOLVED ingress, so a
# forgotten escape hatch still fails here and can never reach paying users.
set -uo pipefail

case "${1:-}" in
  -h | --help)
    sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
esac

SRC="${1:-/dev/stdin}"
if [ "$SRC" != "/dev/stdin" ] && [ ! -f "$SRC" ]; then
  echo "ERROR: plan JSON not found: $SRC" >&2
  exit 2
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "ERROR: python3 not on PATH (needed to parse the plan JSON)" >&2
  exit 2
fi

# Slurp piped input to a temp file BEFORE the python heredoc — the `<<'PY'`
# heredoc takes over python's stdin, so /dev/stdin can't be read inline.
TMP_JSON=""
cleanup() { [ -n "$TMP_JSON" ] && rm -f "$TMP_JSON"; }
trap cleanup EXIT
if [ "$SRC" = "/dev/stdin" ]; then
  TMP_JSON="$(mktemp "${TMPDIR:-/tmp}/alb-plan.XXXXXX.json")"
  cat >"$TMP_JSON"
  SRC="$TMP_JSON"
fi

python3 - "$SRC" <<'PY'
import json
import sys

src = sys.argv[1]
try:
    with open(src) as fh:
        plan = json.load(fh)
except Exception as exc:  # noqa: BLE001 — any parse failure is a hard gate error.
    print(f"ERROR: could not parse plan JSON ({exc})", file=sys.stderr)
    sys.exit(2)

OPEN_CIDRS = {"0.0.0.0/0", "::/0"}
GUARDED_PORTS = {80, 443}

# env defaults to "prod" when the plan does not carry the variable, so an
# unlabeled plan is treated as the strictest (prod) case rather than skipped.
env = (plan.get("variables", {}).get("env") or {}).get("value", "prod")


def alb_ingress_rules(doc):
    """Yield (address, rule) for ingress blocks of the ALB security group only."""
    for change in doc.get("resource_changes", []):
        if change.get("type") != "aws_security_group":
            continue
        # The ALB SG is resource name "alb" in module.alb; skip the service SG
        # (whose only open CIDR is egress, which we never gate on).
        if change.get("name") != "alb":
            continue
        after = (change.get("change") or {}).get("after") or {}
        for rule in after.get("ingress") or []:
            yield change.get("address", "aws_security_group.alb"), rule


seen = 0
offending = []
for address, rule in alb_ingress_rules(plan):
    seen += 1
    ports = {rule.get("from_port"), rule.get("to_port")}
    if not (ports & GUARDED_PORTS):
        continue
    cidrs = set(rule.get("cidr_blocks") or []) | set(rule.get("ipv6_cidr_blocks") or [])
    bad = cidrs & OPEN_CIDRS
    if bad:
        offending.append((address, rule.get("from_port"), rule.get("to_port"), sorted(bad)))

if seen == 0:
    print(
        "ERROR: no ALB security-group ingress found in the plan JSON — cannot "
        "verify origin lockdown (is this the infra/terraform plan?).",
        file=sys.stderr,
    )
    sys.exit(2)

if env != "prod":
    print(f"PASS: env={env!r} — origin-lockdown gate is scoped to prod, not enforced.")
    sys.exit(0)

if offending:
    print("FAIL: prod ALB security group is OPEN to the whole internet:")
    for address, from_port, to_port, bad in offending:
        print(f"  {address} ingress {from_port}-{to_port} -> {', '.join(bad)}")
    print(
        "Set front_with_cloudflare=true so the ALB origin is Cloudflare-CIDR-only "
        "before go-live (see docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md §9)."
    )
    sys.exit(1)

print("PASS: prod ALB ingress on 80/443 is Cloudflare-CIDR-only (no 0.0.0.0/0 or ::/0).")
sys.exit(0)
PY
