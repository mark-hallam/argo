variable "name_prefix" {
  description = "Resource-name prefix, e.g. skycot-prod. The overage task/schedule/roles are named <name_prefix>-overage*."
  type        = string
}

variable "region" {
  description = "AWS region; used for the awslogs driver, the AWS CLI restore, and the constructed ECS cluster ARN."
  type        = string
}

variable "image_repo_url" {
  description = "ECR repo URL the overage task pulls from (same image as the gateway — it bundles scripts/ + the argo binary + AWS CLI)."
  type        = string
}

variable "image_tag" {
  description = "Container image tag the overage task runs. Match the live gateway tag so the bundled scripts/overage-cron.sh + argo binary are in sync."
  type        = string
  default     = "latest"
}

variable "private_subnets" {
  description = "Private subnet IDs the overage task ENIs live in (same subnets as the gateway service)."
  type        = list(string)
}

variable "service_sg_id" {
  description = "Security group the overage task attaches to. Reuse the gateway service SG so the task has the same egress (S3 restore + Stripe API + Secrets Manager)."
  type        = string
}

variable "ecs_cluster_name" {
  description = "Name of the ECS cluster the overage RunTask launches in. The cluster ARN is constructed from this + region + account."
  type        = string
}

variable "execution_role_arn" {
  description = "ECS task-execution role ARN (pull image, fetch secrets, write logs). Reused from the iam module so we do not fork execution IAM."
  type        = string
}

variable "secret_arns" {
  description = "Map of logical secret key -> Secrets Manager ARN. The overage task injects db_key (SKYCOT_DB_KEY_BASE64), stripe_secret_key (STRIPE_SECRET_KEY), and workspace_root_key (ARGO_CREDENTIALS_MASTER_KEY / SKYCOT_WORKSPACE_ROOT_KEY_BASE64). Reused from the secrets module; the execution role already grants GetSecretValue on these."
  type        = map(string)
}

variable "kms_key_arn" {
  description = "CMK ARN for SSE-KMS decrypt of the DB snapshot objects the overage task restores. Empty string ('') = the snapshots are SSE-S3 (AES256) and no KMS grant is added to the task role."
  type        = string
  default     = ""
}

variable "db_snapshot_bucket" {
  description = "S3 bucket holding the gateway's live DB snapshots (DEC-056 B′). The overage task restores the latest snapshot from here before reporting (the live DB is on the gateway task's local disk, only reachable via this snapshot). Same bucket+prefix module.ecs snapshots to. Empty string ('') = the task reads the DB in place (EFS / dev) and the S3 read grant is omitted."
  type        = string
  default     = ""
}

variable "db_snapshot_prefix" {
  description = "S3 key prefix under db_snapshot_bucket where the gateway's live snapshots land (DEC-056). Must match module.ecs's gateway_db_snapshot_prefix so the overage task restores the right snapshots."
  type        = string
  default     = "gateway-live"
}

variable "stripe_price_pro_overage" {
  description = "Stripe metered subscription-item id (si_…) for the Pro overage price (STRIPE_PRICE_PRO_OVERAGE). When set, the scheduled task PUSHES per-tenant overage to Stripe. When empty (default), report-overage runs PLAN-ONLY (no Stripe push) — a safe default until the operator creates the metered price. Set this to turn billing ON."
  type        = string
  default     = ""
}

variable "report_overage_args" {
  description = "Extra flags appended to `argo billing report-overage` (REPORT_OVERAGE_ARGS), e.g. `--dry-run --json` for a no-push preview run. Empty (default) = a real push when stripe_price_pro_overage is also set."
  type        = string
  default     = ""
}

variable "log_retention_days" {
  description = "CloudWatch Logs retention for the overage task log group."
  type        = number
  default     = 30
}

variable "task_cpu" {
  description = "Fargate CPU units for the overage task (256 = 0.25 vCPU; the restore + report is light)."
  type        = number
  default     = 256
}

variable "task_memory" {
  description = "Fargate task memory (MiB) for the overage task. Must satisfy Fargate cpu/memory pairing rules (256 CPU pairs with 512/1024/2048)."
  type        = number
  default     = 512
}

variable "schedule_expression" {
  description = "EventBridge Scheduler expression for the overage run. Default = 18:00 UTC daily (early-morning AEST, off-peak, an hour after the offsite backup so the latest snapshot exists)."
  type        = string
  default     = "cron(0 18 * * ? *)"
}

variable "schedule_timezone" {
  description = "IANA timezone the schedule_expression is evaluated in."
  type        = string
  default     = "UTC"
}

variable "enable_overage_billing_schedule" {
  description = "Whether the daily overage-billing schedule FIRES. The task-def + IAM are always created; this only flips the EventBridge schedule ENABLED/DISABLED. Default true: a fresh apply bills metered overage by default (it is plan-only and harmless until stripe_price_pro_overage is set anyway). Set false to provision the plumbing without firing it."
  type        = bool
  default     = true
}
