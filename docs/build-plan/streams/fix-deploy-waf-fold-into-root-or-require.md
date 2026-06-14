# fix-deploy-waf-fold-into-root-or-require

**Authored against:** origin/main `7f98b30cf` on 2026-06-15
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-deploy-waf-fold-into-root-or-require`
**Priority:** P1 / HIGH — direct-dollar LLM-cost exposure unprotected by default on go-live.

## Problem (VERIFIED 2026-06-15 AWS go-live audit)

The WAF / rate-limit module (`deploy/waf/`, Cloudflare ruleset incl. the genesis LLM-cost
rate-limit) is a **standalone optional module** — `infra/terraform/main.tf` has NO
`module "waf"` (grep = 0 hits), and `AWS_DEPLOY_RUNBOOK.md §9:545-561` labels it
OPTIONAL/standalone. So the go-live `terraform apply` does NOT enforce it; the LLM-cost
rate-limit only exists if the operator runs a SEPARATE third apply. `tofu validate` on
`deploy/waf` passes (the module is sound) — it's just not wired into the go-live path.

**Consequence:** a fresh go-live stack has no rate-limit on the genesis / pitch public
endpoints by default → an abusive caller can run up direct provider ($) cost. This is a
real exposure for a public paying launch, not just defense-in-depth.

## Fix (pick one — recommend (a))
- **(a) Fold `deploy/waf` into the root stack** as `module "waf"` so the same go-live apply
  provisions the rate-limit + firewall rulesets (preferred — one apply, enforced by default).
- **(b)** If WAF must stay a separate Cloudflare-plane apply, add a **mandatory pre-go-live
  gate** (a §12c step + a check script) that verifies `rate_limit_ruleset_id` +
  `firewall_ruleset_id` are live before the cold-smoke, and elevate the WAF apply from
  OPTIONAL to a REQUIRED launch-window step in both runbooks.

## Verifier
- `tofu validate` passes with WAF in the root stack (option a), OR the new pre-go-live gate
  script exits non-zero when the rulesets are absent (option b).
- A documented check that the genesis/pitch public routes are rate-limited before go-live.

## Out of scope
- Tuning the rate-limit thresholds (separate); the WAF ruleset content already exists.

**LOC budget:** ~40–120 (TF module wiring or a gate script + runbook edits).
