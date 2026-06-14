# `backup` module — offsite, versioned, TLS-only S3 backups + daily schedule

Provisions the **independent offsite copy** of the SQLite store that the bundled
`scripts/backup-cron.sh` (shipped via #2867) ships verified snapshots into, plus
the EventBridge schedule + Fargate task that run it daily. EFS gives
filesystem-level durability; a *logical* corruption (bad migration, accidental
wipe) replicates into every EFS snapshot — this bucket is the point-in-time
offsite copy that survives that, and `scripts/restore-drill.sh` proves it
restores.

## What it creates

| Resource | Purpose |
| --- | --- |
| `aws_s3_bucket.backups` (`${name_prefix}-backups`) | the offsite bucket |
| `aws_s3_bucket_versioning` (Enabled) | recover an overwritten/deleted backup |
| `aws_s3_bucket_public_access_block` (all 4 flags `true`) | backups are never internet-reachable |
| `aws_s3_bucket_server_side_encryption_configuration` | SSE-KMS (CMK) when `kms_key_arn` is set, else SSE-S3 |
| `aws_s3_bucket_lifecycle_configuration` | GLACIER after 7d, expire after 35d, reap noncurrent versions after 35d |
| `aws_s3_bucket_policy` | **deny non-TLS access** to the bucket + objects |
| `aws_iam_role.backup_task` (+ inline policy) | runtime role: `s3:Put/Get/List` on **this bucket ARN only** |
| `aws_iam_role.scheduler` (+ inline policy) | scheduler role: `ecs:RunTask` on the backup task + `iam:PassRole` (scoped) |
| `aws_ecs_task_definition.backup` | small Fargate task reusing the gateway image + EFS access point |
| `aws_scheduler_schedule.backup` | daily `cron(0 17 * * ? *)` RunTask |
| `aws_cloudwatch_log_group.backup` | task logs |

Least-privilege: the backup task role's S3 statements are scoped to the bucket
ARN and `${arn}/*` — there is **no `"*"` resource** on any S3 action; the
scheduler's `iam:PassRole` is conditioned to `ecs-tasks.amazonaws.com` and scoped
to the two roles the task uses.

## ✅ Image prerequisite (CLOSED 2026-06-15)

The backup task runs `bash scripts/backup-cron.sh` and uploads with the AWS CLI.
The runtime image now bundles `scripts/` and installs **AWS CLI v2** (root
`Dockerfile` + `deploy/Dockerfile.gateway`, both at parity); its `ENTRYPOINT` is
the `argo` binary (overridden to `/bin/bash` in the task def here). Closed by
`fix-backup-image-bake-scripts-awscli-and-argo-home`.

What is live-on-apply regardless of that follow-up:

* the bucket + versioning + lifecycle + public-access-block + TLS-only policy
  (the data-durability + data-exposure controls), and
* the scoped IAM,

so the operator can run `scripts/backup-cron.sh` **today** from any host that
mounts the EFS volume (runbook §13b compose/systemd path), writing into this
audited bucket — instead of hand-wiring an unaudited bucket with no
`terraform plan` review and no enforced public-access-block.

## ⚠️ `ARGO_HOME` alignment

`backup-cron.sh` snapshots `$ARGO_WORKSPACE/.argo`, so it finds the db only when
the gateway writes it there. The backup task sets `ARGO_HOME=/data/.argo`, and the
**gateway** now does the same (runbook §13b convention) — set both as the image
default AND on the ECS gateway service task def (`modules/ecs/main.tf`, which
**overrides** the image default, so the image change alone is a no-op on ECS).
The empty-db guard in `backup-cron.sh` fails loud rather than shipping a silent
empty backup, so a misalignment is caught, not hidden. ⚠️ This changes the
gateway's data dir: **fresh applies are safe**; an **existing** deploy needs the
one-time migration `backup-cron.sh` prints on the empty-db guard.

## Inputs (selected)

| var | default | notes |
| --- | --- | --- |
| `kms_key_arn` | `""` | `""` => SSE-S3; set to the secrets CMK for SSE-KMS |
| `schedule_expression` | `cron(0 17 * * ? *)` | daily 17:00 UTC |
| `transition_glacier_days` | `7` | |
| `expiration_days` | `35` | current-version expiry |
| `noncurrent_version_expiration_days` | `35` | bounds versioning growth |
| `retention_days` | `14` | local-tarball prune in the script (not remote) |

## Outputs

`backup_bucket_name`, `backup_bucket_arn`, `backup_schedule_arn`,
`backup_task_definition_arn`.

## Gating

Wired in the root `infra/terraform/main.tf` behind `enable_offsite_backups`
(default **`true`** — durable-by-default). A throwaway/dev apply can opt out with
`enable_offsite_backups = false`.

## Restore drill (the durability proof)

```sh
S3_BUCKET=$(terraform -chdir=infra/terraform output -raw backup_bucket_name) \
  SKYCOT_DB_KEY_BASE64=<same at-rest key as the gateway> \
  bash scripts/restore-drill.sh        # latest from S3 -> restore -> boot -> /readyz
```

Local floor (no S3 needed): `bash scripts/restore-drill.sh --local <tarball>`.
