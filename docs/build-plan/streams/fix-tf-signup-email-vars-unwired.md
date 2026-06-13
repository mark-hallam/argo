# fix-tf-signup-email-vars-unwired

**Title**: Wire signup-email Terraform vars (public_base_url + email_outbound + smtp_password) from root into module "ecs"

**Owner**: claude.
**Authored against**: origin/main `306d240523b5643b3863fd9d3494e2e3be76b784` on `2026-06-14`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-tf-signup-email-vars-unwired
**Source**: `docs/skycot/audit/2026-06-14-aws-deploy-readiness.md` — BLOCKER B1 (Terraform IaC) + the closely-related HIGH (smtp_password secret slot).

🔴 **P1 — GO-LIVE BLOCKER. deploy/security-critical → conductor adversarial review before merge; do NOT self-merge.**

## Problem statement (file:line evidence, code-verified on origin/main `306d240`)

The documented AWS deploy path tells the operator to set `public_base_url` + `email_outbound` in
`terraform.tfvars`, but the **root** Terraform module declares neither variable and never threads
them (nor an `smtp_password` secret ARN) into `module "ecs"`. Result: a verbatim runbook paste errors
with `Value for undeclared variable`; omitting them (the only thing that plans cleanly) boots the task
with the email provider defaulting to `noop`, so the very first stranger's magic-link is written
log-only and **no real self-serve user can complete signup**.

Crucial scoping nuance (verified this turn — narrows the fix vs. the audit headline): the **ECS module
already fully supports all three inputs**. Only the **root wiring** is missing. The fix is therefore
root-only (no ECS-module edits).

Evidence:

- **Root vars NOT declared.** `grep -nE 'public_base_url|email_outbound' infra/terraform/variables.tf`
  → exit 1, 0 hits. `grep -rln 'variable "public_base_url"|variable "email_outbound"' infra/terraform/*.tf`
  → NONE_AT_ROOT. The 88-line file (`infra/terraform/variables.tf:1-88`) declares region/env/project/
  domain_name/vpc_cidr/az_count/image_tag/task_cpu/task_memory/log_retention_days/route53_zone_id/
  front_with_cloudflare/alert_email/tags — and stops. No email vars.
- **Root never threads them into ECS.** `infra/terraform/main.tf:88-108` `module "ecs"` passes only
  `secret_arns` + `trust_xff` among the non-infra args — never `public_base_url`, `email_outbound`,
  or an `smtp_password` ARN.
- **No `smtp_password` secret slot.** `infra/terraform/modules/secrets/main.tf:18-47` `secret_specs`
  has bearer_token / workspace_root_key / auth_jwt_secret / db_key / stripe_secret_key /
  stripe_webhook_secret / stripe_price_ids — no `smtp_password`. So even after the var-wiring fix the
  SES password half has no Secrets Manager slot and no IAM read grant (the iam module grants only on
  `module.secrets.secret_arns`).
- **Module support already exists (do NOT rebuild it).** `infra/terraform/modules/ecs/variables.tf:93-109`
  declares `public_base_url` (string, default "") + `email_outbound` (object with optional provider/
  from_address/smtp_host/smtp_port/smtp_username). `infra/terraform/modules/ecs/main.tf:60-78` already
  builds the email env and does `lookup(var.secret_arns, "smtp_password", null)` for
  `SKYCOT_SMTP_PASSWORD`. The gap is purely that the root never feeds these.
- **Runbook instructs the non-existent vars.** `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md:139-154`:
  `# FRONT-DOOR (sign-in email) — set these or the very first user touch breaks:` then
  `public_base_url = ...` and `email_outbound = { ... }`. Secrets step 4 (`:201-249`) populates all 7
  existing secrets via `put-secret-value` but has no `smtp_password` entry.
- **App-side consequence.** With provider=noop, magic-link returns `not_configured` and links are
  log-only (audit cites `crates/argo-cli/src/handlers/gateway/auth_magic_link_delivery.rs:53-57`).
- **`terraform.tfvars.example` also lacks the vars** — operator has no example to copy.

Introduced by commit `f0c124a83` (#2856), which added the ECS-module support + runbook text but not the
root wiring.

## Fix recipe (root-only, additive)

1. **`infra/terraform/variables.tf`** — declare two new root vars matching the ECS-module shape exactly:
   - `public_base_url` — string, `default = ""`. Description: SKYCOT_PUBLIC_BASE_URL public origin the
     SPA is served from; magic-link + invite emails mint links against it.
   - `email_outbound` — object with `optional(string,"")` fields `provider` / `from_address` /
     `smtp_host` / `smtp_port` / `smtp_username`, `default = {}`. Mirror
     `infra/terraform/modules/ecs/variables.tf:99-109` byte-for-byte on the type so the pass-through
     is a clean structural match.
2. **`infra/terraform/main.tf`** `module "ecs"` block (currently `:88-108`) — add:
   `public_base_url = var.public_base_url` and `email_outbound = var.email_outbound`. The
   `smtp_password` ARN flows automatically via the existing `secret_arns = module.secrets.secret_arns`
   pass-through once the secret slot below exists (the ECS module's
   `lookup(var.secret_arns, "smtp_password", null)` picks it up).
3. **`infra/terraform/modules/secrets/main.tf`** — add an `smtp_password` entry to `secret_specs`
   (`:18-47`) following the existing pattern exactly: `id = "skycot-smtp-password"`, a `desc` naming
   `SKYCOT_SMTP_PASSWORD` (the SES SMTP password). It inherits the existing `aws_secretsmanager_secret`
   `for_each` (CMK-encrypted, 7-day recovery) and the `placeholder` version with `REPLACE_ME` +
   `lifecycle { ignore_changes = [secret_string] }`. Because the iam module grants on
   `module.secrets.secret_arns`, the read grant + ARN flow are automatic — no iam-module edit.
4. **`infra/terraform/terraform.tfvars.example`** — add commented `public_base_url` + `email_outbound`
   example lines (mirroring runbook `:139-154`) so a verbatim copy plans cleanly. Keep them commented
   so the default-empty (log-only) path still plans for a bring-up smoke.
5. **`docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md`** — add an `smtp_password` `put-secret-value` step
   alongside the existing 7 (secrets step 4, `:215-249`), targeting
   `--secret-id skycot-prod/skycot-smtp-password`, with a one-line note that the SES SMTP password goes
   here and NEVER into tfvars. Verify the new secret-id name matches the `name_prefix`/`id` the secrets
   module produces (`${var.name_prefix}/${each.value.id}` → `skycot-prod/skycot-smtp-password`).
6. **FEATURE_MATRIX in-band.** Update the relevant AWS-deploy / IaC row in
   `docs/build-plan/FEATURE_MATRIX.md` (status badge + commit SHA + the touched module path) in the
   **same commit** as the code — the markdown table IS the source-of-truth ledger. Do NOT commit the
   generated `docs/skycot/feature-matrix.html` (it is gitignored, per DEC-043).

## Parallel-surface declaration

**No new public Rust types.** This brief touches only Terraform HCL (variables + a locals map entry +
a module argument) and two docs. The lint-brief grep-before-write check targets `pub struct/enum/type`
in `crates/` — N/A here.

For rigor on the Terraform identifiers (no parallel/duplicate HCL declarations created):
- `grep -nE 'public_base_url|email_outbound' infra/terraform/variables.tf infra/terraform/main.tf`
  → 0 hits at root (the names exist ONLY in `modules/ecs/`, which this brief extends-by-wiring, not
  duplicates).
- `grep -rln 'variable "public_base_url"|variable "email_outbound"' infra/terraform/*.tf` → NONE_AT_ROOT.
- `grep -n 'smtp_password' infra/terraform/modules/secrets/main.tf` → 0 hits (slot is genuinely new in
  the secrets module; the matching consumer `lookup(..., "smtp_password", ...)` already exists in
  `modules/ecs/main.tf:75-78`, so this is a WIRE, not an orphan scaffold).

This is the **wire-not-rebuild** outcome the drift playbook wants: the consumer already exists; we add
the producer + the root pass-through.

## May-touch (explicit allowlist)

- `infra/terraform/variables.tf` (declare 2 root vars)
- `infra/terraform/main.tf` (thread 2 vars into `module "ecs"`)
- `infra/terraform/modules/secrets/main.tf` (add `smtp_password` to `secret_specs`)
- `infra/terraform/terraform.tfvars.example` (commented example lines)
- `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md` (add smtp_password put-secret-value step; trim/correct the
  "set these vars" framing if needed)
- `docs/build-plan/FEATURE_MATRIX.md` (in-band ledger row, same commit)

**Do NOT touch**: any `crates/` Rust (the app side already behaves correctly), `modules/ecs/`
(support already complete), `modules/iam/` (grant flows automatically via `secret_arns`),
`modules/alb/` (B2 origin-lockdown is a SEPARATE blocker/brief), or any other module.

## Acceptance criteria (CHECKABLE verifier)

- [ ] `terraform -chdir=infra/terraform validate` exits 0 (run after `terraform init -backend=false`
      if no backend creds locally). Paste the tail in the PR body.
- [ ] **Undeclared-variable gone**: write the runbook's documented tfvars block (the verbatim
      `public_base_url = "https://app.skycot.com"` + the full `email_outbound = { provider="smtp" ... }`
      from `AWS_DEPLOY_RUNBOOK.md:139-154`) into a temp tfvars and run
      `terraform -chdir=infra/terraform plan -var-file=<that-file>` (or `validate` with the vars set) —
      it must NOT emit `Value for undeclared variable`. Paste proof in the PR body. (A plan needing AWS
      creds may stop at provider auth — that is FINE; the pass condition is the absence of the
      undeclared-variable error, which surfaces during variable evaluation BEFORE provider calls.)
- [ ] **Email provider boots non-noop**: prove the wired env reaches the container as a non-noop
      provider. Acceptable evidence (pick the one that runs without live AWS):
      - `terraform -chdir=infra/terraform plan` (vars set as above) and grep the rendered
        `aws_ecs_task_definition.this` `container_definitions` for
        `"name":"SKYCOT_EMAIL_PROVIDER","value":"smtp"` AND a `SKYCOT_SMTP_PASSWORD` entry under
        `secrets` whose `valueFrom` is the new `skycot-prod/skycot-smtp-password` ARN; OR
      - a `terraform console`/targeted output showing `module.ecs` receives `email_outbound.provider ==
        "smtp"` and the smtp_password ARN is present in `secret_arns`. Paste in the PR body.
- [ ] `bash scripts/lint-brief.sh fix-tf-signup-email-vars-unwired` reports ok (or only the known
      WARN-mode token-overlap noise against the unrelated web brief
      `fix-signup-no-email-required-button-honest-copy`).
- [ ] `docs/build-plan/FEATURE_MATRIX.md` updated in the SAME commit (status + SHA + module path).
- [ ] All touched files stay <500 LOC; no fn >80 LOC (N/A for HCL but stated for the rule).

## Out of scope (separate briefs / blockers)

- B2 — ALB origin internet-open by default (`front_with_cloudflare` prod default + origin-lockdown
  sequencing). Separate blocker.
- B3 — stand the stack up on AWS credits and run the cold-stranger smoke + `fresh-boot-loop.sh 10`.
  This brief makes signup-email *wirable*; proving it works end-to-end in the cloud is B3's job.
- Non-root container USER, offsite-backup Terraform module, CI re-enable — all separate punch-list rows.

## Effort

Small. ~30-50 LOC of HCL + ~15 lines of docs across 6 files. ~1 hour including the validate/plan
verifier evidence.