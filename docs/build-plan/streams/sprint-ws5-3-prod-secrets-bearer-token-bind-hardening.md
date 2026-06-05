# sprint-ws5-3-prod-secrets-bearer-token-bind-hardening

**Owner**: claude (Windows lane B — backend+infra lane).
**Depends on**: sprint-ws5-1-aws-iac-single-binary-gateway (#2683 — creates `infra/terraform/modules/secrets/`; this brief hardens it). **Stacked on the WS5-1 branch until #2683 merges.**
**Authored against**: origin/main `0ee90d905` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws5-3-prod-secrets-bearer-token-bind-hardening`.

## Why

`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS5-3) calls for: *"wire bearer
token + master key + Stripe/provider secrets into AWS Secrets Manager; confirm
`ARGO_GATEWAY_PROFILE=production` forces auth_bypass off."*

WS5-1 (#2683) created the 6 Secrets Manager entries (bearer / master-key / JWT / Stripe
secret + webhook + price-ids) as placeholders. WS5-3 **hardens** that wiring to an
enterprise-grade posture and adds the operator-facing **canonical secrets template**, plus
**confirms** the production auth-bypass guarantee with code-grounded evidence.

Two gaps WS5-3 closes:
1. **Secrets encryption + lifecycle.** WS5-1's secrets use the AWS-managed default key and
   the 30-day deletion recovery window. For the governance-moat enterprise tier, secrets
   should be encrypted with a **customer-managed KMS key (CMK)** whose decrypt grant is
   scoped to the task-execution role, with a short, explicit recovery window.
2. **No canonical production-secret template.** The three existing env templates
   (`deploy/.env.deploy.example`, `deploy/docker/.env.example`, `deploy/flyio/secrets.example.sh`)
   are each partial AND one names the **wrong** JWT env var (`deploy/flyio/secrets.example.sh:14`
   uses `JWT_SIGNING_KEY`; the code reads `SKYCOT_AUTH_JWT_SECRET` —
   `middleware_session_verify.rs:53`). There is no single code-grounded list of every secret
   a production deploy needs.

## What

### 1. KMS CMK for secrets (new `modules/kms/` + edit `modules/secrets/`)

- New `infra/terraform/modules/kms/` — an `aws_kms_key` + `aws_kms_alias`
  (`alias/<prefix>-secrets`) with rotation enabled and a key policy that:
  - allows the account root full admin (so the operator isn't locked out), and
  - allows `secretsmanager` service usage in-region.
- Edit `modules/secrets/`: accept a `kms_key_arn` variable and set `kms_key_id` on every
  `aws_secretsmanager_secret`; add `recovery_window_in_days = 7`.
- Edit root `main.tf`: wire `module.kms` → `module.secrets`; the IAM execution role's
  decrypt is already implicit via `secretsmanager:GetSecretValue` + the key policy, but
  add an explicit `kms:Decrypt` on the CMK ARN to the `iam` module's scoped policy so a
  CMK (not the default key) is honored.

### 2. Canonical production secrets template (new file)

- New `deploy/secrets/skycot-production.env.example` — the SINGLE code-grounded list of
  every production secret/config the gateway + billing need, each with a placeholder, the
  exact env var name **verified against the crate that reads it**, a generate-command where
  applicable, and a one-line purpose. Includes `ARGO_GATEWAY_PROFILE=production`.
- Header cross-references: the AWS path sets these via Secrets Manager (runbook §6); the
  docker/VM paths inject this `.env` directly; it supersedes the partial templates for the
  full set. Fix-forward note on the Fly.io `JWT_SIGNING_KEY` misnomer.

### 3. Confirm `ARGO_GATEWAY_PROFILE=production` forces auth_bypass off (verification)

- Document the code-grounded proof (boot gate + request gate) in the PR + a short
  `docs/skycot/deploy/` note. The integration test
  `production_profile_without_bearer_fails_before_listen` already asserts the boot-refuse
  path.
- **Windows-lane caveat (NO THEATER):** `argo-cli` does not compile on the Windows Account-B
  box (`bergshamra-* → kryptering → cryptoki 0.7.0` build break), so the test is
  VERIFIED-BY-CODE-READ here, not VERIFIED-BY-TEST-RUN. The Mac/Linux swarm runs the real
  test in its existing suite.

## May-touch
- `infra/terraform/modules/kms/**` (NEW).
- `infra/terraform/modules/secrets/{main.tf,variables.tf,README.md}` (edit — add CMK + recovery window).
- `infra/terraform/modules/iam/{main.tf,variables.tf}` (edit — add scoped `kms:Decrypt`).
- `infra/terraform/main.tf` + `outputs.tf` (wire kms module).
- `deploy/secrets/skycot-production.env.example` (NEW).
- `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md` (edit — note the CMK + reference the template).
- `docs/build-plan/BUILD_STATUS.md` (ledger flip on PR open).

Do NOT touch any Rust crate (production-gate "confirm" is read-only verification; the
`authenticate_request` defense-in-depth hardening is flagged as a separate Mac-lane task).

## Acceptance criteria
- [ ] `cd infra/terraform && terraform init -backend=false && terraform validate` exits 0.
- [ ] `terraform fmt -recursive -check infra/terraform` exits 0.
- [ ] Every secret in `modules/secrets` carries `kms_key_id = <CMK>` + `recovery_window_in_days = 7`.
- [ ] `modules/kms` has README + variables + outputs; CMK has rotation enabled + a scoped key policy.
- [ ] IAM execution-role policy includes `kms:Decrypt` scoped to the CMK ARN (no wildcard).
- [ ] `deploy/secrets/skycot-production.env.example` lists ALL of: `ARGO_GATEWAY_PROFILE`,
      `ARGO_GATEWAY_BEARER_TOKEN`, `SKYCOT_WORKSPACE_ROOT_KEY_BASE64` /
      `ARGO_CREDENTIALS_MASTER_KEY`, `SKYCOT_AUTH_JWT_SECRET`, `STRIPE_SECRET_KEY`,
      `STRIPE_WEBHOOK_SECRET`, the price-id config, `SKYCOT_DB_KEY_BASE64`, `ARGO_HOME` —
      each env var name verified against the reading crate.
- [ ] PR body documents the production-gate proof (boot.rs + routes_auth.rs citations) AND
      the honest Windows-cannot-run-the-test caveat.
- [ ] No file > 500 LOC; no Rust file changed.

## Out of scope
- The `authenticate_request` production-bypass code fix (flagged as a Mac-lane spawn task).
- Secret rotation automation (Lambda rotation) — note as a post-launch follow-up.
- A live `terraform apply` / real secret values (operator-gated).
- WS5-4 (WAF + LLM-spend cap) — separate brief.

## Parallel-surface check
- `rg 'modules/kms' infra/` — none today; this brief creates it.
- `rg 'aws_kms_key' .` — none today.
- `deploy/secrets/` — new namespace (no existing env file there); the 3 partial templates
  stay; this is the canonical superset, cross-referenced, not a silent parallel.

## Source
`docs/skycot/audit/2026-06-04-day0-sprint-gap-audit.md` (WS5-3). Sprint playbook
`docs/skycot/SPRINT-paying-ready-2026-06-04.md` (WS5; AWS Sydney). Backend/infra-lane
Windows lane B sprint mandate.
