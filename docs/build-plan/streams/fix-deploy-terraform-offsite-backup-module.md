# fix-deploy-terraform-offsite-backup-module

**Authored against**: origin/main `306d240523b5643b3863fd9d3494e2e3be76b784` on `2026-06-14`
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-deploy-terraform-offsite-backup-module

**Owner:** claude

> **deploy/security-critical → conductor adversarial review before merge; do NOT self-merge.** This brief touches the production IaC (a new S3 data bucket + an IAM-granted scheduled RunTask) and is data-durability-critical. It must pass conductor adversarial review (correct lifecycle, public-access-block, least-privilege IAM, no internet-exposed bucket) before it lands. Self-merge is forbidden.

## Goal

Promote the runbook §13b offsite-backup wiring into the root Terraform so a fresh
`terraform apply` yields offsite, versioned, lifecycle'd, public-access-blocked
S3 backups **by default** — running `scripts/backup-cron.sh` daily on an
EventBridge-scheduled ECS RunTask — and prove it restores. Today a fresh apply
has EFS-snapshot durability ONLY; a logical corruption (bad migration, accidental
wipe) replicates into every EFS snapshot with no independent offsite copy.

## Problem statement (file:line evidence, all verified on origin/main `306d240523b`)

1. **The only backup in the IaC is the EFS backup policy.** `infra/terraform/modules/efs/main.tf:42-48` declares `aws_efs_backup_policy "this" { backup_policy { status = "ENABLED" } }` and nothing else. `rg -n 'backup' infra/terraform/` returns only that file + READMEs; `rg -n 'aws_scheduler|cloudwatch_event|RunTask|run_task' infra/terraform/` returns **0 hits**. There is no S3 backup bucket, no schedule, no RunTask in the IaC.
2. **The root module wires 8 modules, none for backup.** `infra/terraform/main.tf:29-119` instantiates `network`, `ecr`, `efs`, `kms`, `secrets`, `iam`, `alb`, `ecs`, `observability` — there is no `module "backup"`.
3. **The runbook itself confirms the wiring is unfinished and operator-manual.** `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md:685-726` (§13b) tells the operator to run `scripts/backup-cron.sh` daily "either as an EventBridge-scheduled ECS RunTask … or via the compose/systemd units," to add an S3 lifecycle rule (Glacier 7d / expire 35d) and to run the restore drill — all by hand. The "What this runbook does NOT cover" caveat at `AWS_DEPLOY_RUNBOOK.md:799-803` states plainly: *"the Terraform wiring of the scheduled task / S3 bucket lifecycle is still operator-run, not in this module."*
4. **The application-level scripts already ship and are verified — DO NOT re-author them.** `scripts/backup-cron.sh` (create→verify→`aws s3 cp`→prune; env `S3_BUCKET`/`S3_PREFIX`/`ARGO_WORKSPACE`/`RETENTION_DAYS`/`INCLUDE_CREDENTIALS`, AWS auth via task role) and `scripts/restore-drill.sh` (latest-from-S3 → restore → boot → `/readyz`; env `S3_BUCKET`/`SKYCOT_DB_KEY_BASE64`) shipped via the MERGED brief `launch-backup-cron-s3-offsite-and-restore-drill` (#2867, commit `4a31f1136`). **This brief is the IaC counterpart of that brief — it provisions the bucket + schedule + IAM those scripts target. It must not duplicate or modify the scripts' logic.**

Consequence: until this lands, every operator who follows the runbook either skips offsite backups (EFS-only durability) or hand-wires an unaudited bucket + schedule with no `terraform plan` review and no enforced public-access-block — a P1 data-durability and data-exposure risk for paying users.

## Fix (strongly preferred: a Terraform module, so `apply` is durable-by-default)

1. **New module `infra/terraform/modules/backup/`** (`main.tf` + `variables.tf` + `outputs.tf` + `README.md`):
   - `aws_s3_bucket` `${name_prefix}-backups` with: `aws_s3_bucket_versioning` (Enabled); `aws_s3_bucket_lifecycle_configuration` (transition to `GLACIER` after 7 days, `expiration` after 35 days, plus a noncurrent-version expiration so versioning does not grow unbounded); `aws_s3_bucket_public_access_block` (all four flags `true`); `aws_s3_bucket_server_side_encryption_configuration` (SSE — reuse the CMK from `module.kms` if passed, else SSE-S3); and `aws_s3_bucket_policy` denying non-TLS access. Default-tagged via the provider block (no new tags needed).
   - An EventBridge schedule running the backup daily — prefer `aws_scheduler_schedule` (EventBridge Scheduler) with `target { ecs_parameters { task_definition_arn = … task_count = 1 launch_type = "FARGATE" network_configuration { … private subnets, service SG } } }` and `command`/container override invoking `bash scripts/backup-cron.sh` with `S3_BUCKET`/`ARGO_WORKSPACE=/data`/`RETENTION_DAYS` env, mounting the same EFS access point as the gateway task. A separate small backup task definition reusing the same image is acceptable; reuse `module.ecs`'s cluster + task-exec role rather than forking IAM.
   - A least-privilege IAM policy granting the backup task role `s3:PutObject`/`s3:GetObject`/`s3:ListBucket` **scoped to this bucket ARN only** (no wildcards) and the EventBridge Scheduler role `ecs:RunTask` + `iam:PassRole` scoped to the backup task's roles only.
   - Outputs: `backup_bucket_name`, `backup_bucket_arn`, `backup_schedule_arn`.
2. **Wire it into the root** at `infra/terraform/main.tf` (a new `module "backup"` block after `module "ecs"`), passing `name_prefix`, the EFS file-system + access-point ids (`module.efs.*`), private subnets + service SG (`module.network.*` / `module.alb.service_sg_id`), the ECS cluster name + image (`module.ecs.*` / `module.ecr.repository_url`), and the CMK (`module.kms.key_arn`). Gate behind a root var `enable_offsite_backups` (default **`true`** for prod — durable-by-default is the whole point; expose so a throwaway/dev apply can opt out).
3. **Add the bucket name + schedule ARN to `infra/terraform/outputs.tf`** so the operator can run the restore drill against the real bucket without hand-typing.
4. **Update the runbook §13b + the "does NOT cover" caveat** (`AWS_DEPLOY_RUNBOOK.md:685-726,799-803`) to reflect that the bucket + lifecycle + scheduled RunTask are now provisioned by the root apply; keep the manual systemd/compose path documented as the non-AWS alternative; keep the restore-drill-is-mandatory instruction.
5. **Update `docs/build-plan/FEATURE_MATRIX.md` in-band, same commit** — flip/append the offsite-backup IaC row to ✅ Live with this brief's commit SHA + `infra/terraform/modules/backup` as the source (the ledger markdown IS the source of truth; do NOT commit the generated `feature-matrix.html`).

Keep each new `.tf` file < 500 LOC (these are small; split bucket vs schedule vs iam if any file approaches the ceiling). No Rust touched, so the <80 LOC/fn rule does not bind here.

## Explicit may-touch list

- `infra/terraform/modules/backup/main.tf` (NEW)
- `infra/terraform/modules/backup/variables.tf` (NEW)
- `infra/terraform/modules/backup/outputs.tf` (NEW)
- `infra/terraform/modules/backup/README.md` (NEW)
- `infra/terraform/main.tf` (add `module "backup"` block only)
- `infra/terraform/variables.tf` (add `enable_offsite_backups` var only)
- `infra/terraform/outputs.tf` (add backup bucket + schedule outputs only)
- `infra/terraform/terraform.tfvars.example` (document the new var)
- `docs/skycot/deploy/AWS_DEPLOY_RUNBOOK.md` (§13b text + "does NOT cover" caveat only)
- `docs/build-plan/FEATURE_MATRIX.md` (the backup row, in-band same commit)

**Do NOT touch:** `scripts/backup-cron.sh`, `scripts/restore-drill.sh`, `scripts/backup-consistency-test.sh` (shipped + verified via #2867 — this brief consumes them, it does not change them); `modules/efs/` (the EFS backup policy stays as-is); `modules/iam/` and `modules/secrets/` (the backup module carries its own scoped IAM; do not widen the gateway task role).

## Parallel-surface declaration

No new public Rust types — this brief is Terraform + docs only. Greps run on origin/main `306d240523b`:
- `rg -n 'aws_scheduler|cloudwatch_event|RunTask|run_task' infra/terraform/` → **0 hits** (no existing schedule/RunTask to collide with).
- `rg -n 'backup' infra/terraform/` → only `modules/efs/main.tf:42-48` (EFS backup policy) + READMEs. **No existing S3 backup bucket or `module "backup"`.**
- `ls infra/terraform/modules/ | rg -i backup` → empty (no `backup` module dir exists; creating it is additive, not parallel).
- Brief-id note: the MERGED brief `launch-backup-cron-s3-offsite-and-restore-drill` (#2867) shares the `backup`/`offsite`/`restore` tokens but is the **scripts** layer (now shipped); this brief is the **IaC** layer it targets. They are sequential, not parallel — this brief explicitly consumes #2867's scripts and modifies no script. The chosen id (`…terraform-offsite-backup-module`) carries the distinguishing `terraform`/`module` scope to avoid a parallel-name false-positive.

## Acceptance criteria (CHECKABLE verifier — NO THEATER)

1. **`terraform -chdir=infra/terraform validate`** exits 0 with the new module wired.
2. **`terraform -chdir=infra/terraform plan`** (against a configured backend, or `plan -refresh=false` with placeholder vars) shows the plan **creating** all of: an `aws_s3_bucket` `${name_prefix}-backups`, its `aws_s3_bucket_versioning` (Enabled), its `aws_s3_bucket_lifecycle_configuration` (Glacier 7d / expire 35d), its `aws_s3_bucket_public_access_block` (all four `true`), and the scheduled `aws_scheduler_schedule` (or `aws_cloudwatch_event_rule` + `aws_cloudwatch_event_target`) RunTask. Paste the relevant `plan` lines in the PR body. (`grep -c 'will be created' ` is a quick sanity floor.)
3. **`terraform fmt -check -recursive infra/terraform/modules/backup`** clean; **`terraform validate`** clean.
4. **Public-access assertion:** the plan/state shows `block_public_acls`, `block_public_policy`, `ignore_public_acls`, `restrict_public_buckets` all `true` — assertable via `terraform show -json | jq` or by grepping the plan; no path makes the bucket public.
5. **IAM least-privilege assertion:** the backup task-role policy resource is scoped to the new bucket ARN only (no `"*"` resource on S3 actions) — show the policy JSON in the PR.
6. **Restore-drill against the bucket succeeds** (the durability proof, per the FIX): after apply on the AWS credits (or, if cloud apply is deferred to the B3 throwaway-apply, document UNVERIFIED-IN-CLOUD honestly and run the local proof), `S3_BUCKET=<the new bucket> SKYCOT_DB_KEY_BASE64=<key> bash scripts/restore-drill.sh` emits `{"ok":true,…}` with `/readyz` status==ok and a positive `schema_version`. CI-equivalent local floor when cloud is deferred: `bash scripts/restore-drill.sh --local <tarball>` → `ok=true` (proves the consuming script still works against the layout this module provisions).
7. `docs/build-plan/FEATURE_MATRIX.md` updated in the **same commit** with the commit SHA + `infra/terraform/modules/backup` source.
8. `bash scripts/lint-brief.sh fix-deploy-terraform-offsite-backup-module` reports ok (or only the known token-overlap WARN, which this declaration explains).

## Size / owner

~150-250 LOC of HCL across 3-4 small `.tf` files + docs. No Rust crates touched. Owner: claude. 🔴 P1 DATA DURABILITY — a fresh AWS apply currently has EFS-snapshot durability ONLY; this makes offsite, versioned, lifecycle'd, public-access-blocked backups the default and proves restore. **deploy/security-critical → conductor adversarial review before merge; do NOT self-merge.**