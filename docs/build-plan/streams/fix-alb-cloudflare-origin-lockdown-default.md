# fix-alb-cloudflare-origin-lockdown-default

**Owner**: claude — **P1 BLOCKER · deploy/security-critical (AWS deploy-readiness audit B2).**
**Depends on**: none. (Sibling to `fix-` for B1 email-wiring + B3 cloud-apply, but independently shippable.)
**Authored against**: origin/main `306d240523b5643b3863fd9d3494e2e3be76b784` on `2026-06-14`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-alb-cloudflare-origin-lockdown-default

Source: `docs/skycot/audit/2026-06-14-aws-deploy-readiness.md` — BLOCKER **B2** ("ALB origin is internet-open by default; Cloudflare WAF + rate-limits are bypassable").

---

## Problem (file:line evidence, CONFIRMED on origin/main `306d2405` this turn)

The Application Load Balancer (ALB — the AWS front door that terminates TLS and routes traffic to the gateway) ships **open to the entire internet by default**, so every Cloudflare and Web-Application-Firewall (WAF — the edge filter that rate-limits and blocks abusive requests) protection is **bypassable at go-live**.

The chain, verified by reading each file on current `origin/main`:

1. `infra/terraform/variables.tf:72-76` — `variable "front_with_cloudflare"` has `default = false`.
2. `infra/terraform/modules/alb/main.tf:4-7` — when `front_with_cloudflare` is false, the security-group ingress is set wide open:
   ```hcl
   locals {
     alb_ingress_v4 = var.front_with_cloudflare ? var.cloudflare_ipv4_cidrs : ["0.0.0.0/0"]
     alb_ingress_v6 = var.front_with_cloudflare ? var.cloudflare_ipv6_cidrs : ["::/0"]
   }
   ```
   These locals feed the `aws_security_group "alb"` ingress on ports 80 + 443 (`modules/alb/main.tf:9-26`). The ALB is `internet-facing` and its DNS name is a non-secret Terraform output.
3. `infra/terraform/main.tf:77-86` — the root `module "alb"` passes `front_with_cloudflare = var.front_with_cloudflare` (so the default `false` flows straight through; the Cloudflare CIDR lists are NOT passed from root — they live as module defaults at `modules/alb/variables.tf:45-79`, 15 IPv4 + 7 IPv6 ranges).
4. `infra/terraform/main.tf:104-108` (the `module "ecs"` block) — also threads `trust_xff = var.front_with_cloudflare`, so when the origin is open, `ARGO_TRUST_XFF` is off and the gateway's per-IP limiter keys on a header it does NOT yet trust; once XFF trust is turned on without the SG lockdown, that limiter becomes **spoofable** via `X-Forwarded-For`.
5. `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md:477-516` (§9) documents the lockdown as **"OPTIONAL / LATER, default-off"** and a **"SECOND, separate `terraform apply`"** — **nothing forces it before anonymous/paying users arrive.**

**Why this is a BLOCKER:** every edge control — Cloudflare rate-limits, the `deploy/waf/` genesis LLM-cost block (direct dollar exposure), bot challenges — applies ONLY to traffic that traverses Cloudflare's proxy. An attacker who resolves the public ALB DNS name hits the origin directly and bypasses **all of it**, including the cost guard. The default operator path (paste `terraform.tfvars.example`, `terraform apply`) yields an internet-open origin with zero edge protection.

---

## May-touch list (explicit — stay inside these)

- `infra/terraform/variables.tf` — change the default + clarify the description of `front_with_cloudflare`.
- `infra/terraform/main.tf` — if a validation/gate is wired at the root, the precondition/check block goes here.
- `infra/terraform/modules/alb/main.tf` — OPTIONAL: add a Terraform-native `precondition`/`check` block that fails plan/apply when the resolved ingress CIDR is `0.0.0.0/0` and `env == "prod"` (see "Approach", option chosen below).
- `infra/terraform/modules/alb/variables.tf` — only if a new `env`/`require_cloudflare_origin` toggle is needed to scope the gate to prod.
- `scripts/check-alb-origin-locked.sh` — **NEW** pre-deploy hard-gate script (see acceptance).
- `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md` — re-sequence §9 so cutover → Full(Strict) → origin-lockdown happen in **one launch window**, and add the §12 pre-deploy hard-gate step.
- `docs/build-plan/FEATURE_MATRIX.md` — update the matching row **in-band, same commit** (see reminder below).

Do NOT touch any Rust crate, the gateway code, the WAF module, or the email/secrets wiring (separate B1/HIGH briefs). Do NOT widen scope to the dynamic-Cloudflare-CIDR refresh (that is the MEDIUM "source CIDRs dynamically" finding — leave it as a follow-up).

---

## Parallel-surface declaration

**No new public Rust types.** This brief changes a Terraform variable default + adds one shell script + edits one runbook + updates the feature matrix. There is no Rust `pub struct`/`enum`/`type`.

`rg` results run this turn:
- `rg -l 'alb-cloudflare|cloudflare-origin|origin-lockdown' docs/build-plan/streams/` → **0 hits** (no parallel brief).
- `grep -iE 'alb.cloudflare|cloudflare.origin|origin.lockdown' docs/build-plan/BUILD_STATUS.md` → **0 hits** (no parallel ledger row).
- New script path `scripts/check-alb-origin-locked.sh` — confirm uniqueness at author time with `ls scripts/check-alb-origin-locked.sh` (expected: not present). It follows the existing `scripts/check-*.sh` naming convention; verify no same-purpose script exists (`rg -l '0\.0\.0\.0/0' scripts/` returned nothing this turn).

---

## Approach (first-principles: make the safe state the DEFAULT, and make the unsafe state FAIL LOUD)

The root cause is that the *convenient* default (open origin for bring-up smoke) is also the *go-live* default. The fix makes the secure posture the default for prod and adds a gate that refuses to ship an open origin — so an operator can't reach paying users via the documented path without the lockdown.

1. **Default the secure posture for prod.** Flip `front_with_cloudflare` so that a prod apply restricts the ALB SG to Cloudflare CIDRs by default. Because flipping the bare default to `true` would black-hole the *bring-up* smoke (the SG would reject the operator's own non-Cloudflare request before DNS is proxied), gate the default on environment: keep `false` only for non-prod / explicit bring-up, and either (a) set `default = true` and document a one-line `front_with_cloudflare = false` override for the bring-up window, OR (b) add an `env`-scoped Terraform `precondition` (preferred — see step 2) that forces the operator to consciously opt out for prod. **Recommended: do BOTH** — flip the default to `true` AND add the precondition so the open state is impossible to reach silently in prod.

2. **Add a Terraform-native hard-gate.** In `modules/alb/main.tf` (or root `main.tf`), add a `lifecycle { precondition { ... } }` (or a `check` block) that fails `plan`/`apply` when `var.env == "prod"` and the resolved ingress is `["0.0.0.0/0"]` — error message: `"prod ALB SG is open to 0.0.0.0/0 — set front_with_cloudflare=true so the origin is Cloudflare-CIDR-only before go-live"`. This is the in-Terraform safety net.

3. **Add a standalone pre-deploy shell hard-gate** at `scripts/check-alb-origin-locked.sh` — usable in the runbook §12 and in CI. It must parse `terraform plan -json` (or `terraform show -json` of the plan file) and exit **non-zero** if any `aws_security_group` rule on the ALB SG resolves to `0.0.0.0/0` or `::/0` on ports 80/443 for the prod env. Pure-bash + `jq`-or-`python3`-stdlib; <80 LOC; no network calls (operates on the local plan JSON). Print the offending rule on failure.

4. **Re-sequence the runbook.** Rewrite `AWS_DEPLOY_RUNBOOK.md` §9 so origin-lockdown is part of the SAME launch window as cutover + Full(Strict), not a "LATER" second apply; add a §12 pre-deploy gate step that runs `scripts/check-alb-origin-locked.sh` and refuses go-live on a non-zero exit. Keep the truthful bring-up note (open is acceptable as a *transient* pre-DNS state, never as the go-live state).

Respect repo invariants: <500 LOC/file, <80 LOC/fn (the gate script and any helper functions).

---

## Acceptance criteria (each must have a CHECKABLE verifier)

1. **Secure-by-default plan.** `cd infra/terraform && terraform plan` with the prod defaults (no `front_with_cloudflare` override) shows the ALB security group ingress on 80/443 restricted to the Cloudflare CIDR lists (15 IPv4 + 7 IPv6 from `modules/alb/variables.tf`), NOT `0.0.0.0/0`/`::/0`. **Verifier:** `terraform plan -no-color | grep -A3 'aws_security_group.*alb'` (or a `-json` plan) shows the Cloudflare CIDRs and contains NO `0.0.0.0/0` on the ALB SG ingress for prod.
2. **Open-origin fails the hard-gate.** With `front_with_cloudflare = false` forced on a prod env, `bash scripts/check-alb-origin-locked.sh <plan.json>` exits **non-zero** and prints the offending `0.0.0.0/0` rule. With the secure default, it exits **0**. **Verifier:** two `terraform plan -out` runs (one forced-open, one default) piped through the gate script; assert exit 1 then exit 0. Capture both exit codes in the PR body.
3. **terraform validate passes.** `terraform validate` (and `terraform fmt -check`) is clean after the variable/precondition edits. **Verifier:** command exit 0 in PR body.
4. **Transient bring-up still possible.** The brief preserves an explicit, documented one-line override (`front_with_cloudflare = false`) for the pre-DNS bring-up smoke, so step-13 smoke is not blocked. **Verifier:** runbook §9 + §13 read coherently; the gate is scoped to prod go-live, not bring-up.
5. **Runbook re-sequenced.** `AWS_DEPLOY_RUNBOOK.md` §9 + §12 show cutover → Full(Strict) → origin-lockdown in one launch window, with the pre-deploy gate step present. **Verifier:** grep the runbook for the §12 gate step and the removed "OPTIONAL / LATER" framing on the go-live path.

(Cloud-side confirmation — `curl` direct-to-ALB-DNS from a non-Cloudflare IP returning a timeout — belongs to the B3 "apply once on credits" brief; this brief's verifiers are plan-time + gate-script, which is the right CHECKABLE bar for an IaC-default change that cannot be cloud-applied in-band.)

---

## Ledger discipline (in-band, same commit — MANDATORY)

Update the matching row in `docs/build-plan/FEATURE_MATRIX.md` in the **same commit** as the code (status badge + commit SHA + crate/route per the source-of-truth-ledger rule). Do NOT commit the generated `docs/skycot/feature-matrix.html` (DEC-043 — it is a gitignored on-demand artifact; regenerate locally only to view via `python3 scripts/render-feature-matrix.py`). If no AWS-edge-security row exists yet, add one for the ALB origin-lockdown posture.

---

## Review gate (NON-NEGOTIABLE)

**This is deploy/security-critical — it changes deploy behavior (the default ALB exposure posture) → conductor adversarial review before merge; do NOT self-merge.** The conductor must confirm: (a) the prod default genuinely produces a Cloudflare-CIDR-only SG in a fresh `terraform plan`; (b) the bring-up override still works so step-13 smoke is not black-holed; (c) the pre-deploy gate script actually fails on a forced-open plan (run it both ways); (d) the runbook no longer routes an operator to a go-live with an open origin.
