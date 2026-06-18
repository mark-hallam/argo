# Edge WAF + per-IP rate-limiting (rate_limit + custom-firewall rulesets) for the
# public API — folded into the DNS apply so it is ENFORCED BY DEFAULT. The
# operator already runs `cd deploy/dns && terraform apply` to point the domain at
# the stack; this makes that same apply also provision the rate-limit (no
# separate `deploy/waf` apply to remember). The genesis LLM-cost block + auth/api
# per-IP caps live in the reusable ../waf module; only zone_id is required, the
# rate-limit thresholds use the module's defaults (tune via the module vars).
module "waf" {
  source  = "../waf"
  zone_id = var.zone_id
}
