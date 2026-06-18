variable "name_prefix" {
  description = "Resource-name prefix, e.g. skycot-prod. The backup bucket is named <name_prefix>-backups."
  type        = string
}

variable "region" {
  description = "AWS region; used for the awslogs driver + the constructed ECS cluster ARN."
  type        = string
}

variable "image_repo_url" {
  description = "ECR repo URL the backup task pulls from (same image as the gateway)."
  type        = string
}

variable "image_tag" {
  description = "Container image tag the backup task runs."
  type        = string
  default     = "latest"
}

variable "private_subnets" {
  description = "Private subnet IDs the backup task ENIs live in (same subnets as the gateway service)."
  type        = list(string)
}

variable "service_sg_id" {
  description = "Security group the backup task attaches to. Reuse the gateway service SG so the task can reach EFS mount targets."
  type        = string
}

variable "ecs_cluster_name" {
  description = "Name of the ECS cluster the backup RunTask launches in. The cluster ARN is constructed from this + region + account."
  type        = string
}

variable "execution_role_arn" {
  description = "ECS task-execution role ARN (pull image, fetch secrets, write logs). Reused from the iam module so we do not fork execution IAM."
  type        = string
}

variable "efs_file_system_id" {
  description = "EFS filesystem id holding /data (the SQLite store). The backup task mounts the SAME filesystem as the gateway to snapshot the live db."
  type        = string
}

variable "efs_access_point_id" {
  description = "EFS access point id the backup task mounts at /data (same access point as the gateway)."
  type        = string
}

variable "secret_arns" {
  description = "Map of logical secret key -> Secrets Manager ARN. The backup task injects `db_key` as SKYCOT_DB_KEY_BASE64 so `argo` can open the at-rest-encrypted store. Reused from the secrets module; the execution role already grants GetSecretValue on these."
  type        = map(string)
}

variable "kms_key_arn" {
  description = "CMK ARN for SSE-KMS bucket encryption + S3 object crypto by the task. Empty string ('') falls back to SSE-S3 (AES256) and skips the KMS grant on the task role."
  type        = string
  default     = ""
}

variable "log_retention_days" {
  description = "CloudWatch Logs retention for the backup task log group."
  type        = number
  default     = 30
}

variable "task_cpu" {
  description = "Fargate CPU units for the backup task (512 = 0.5 vCPU; the snapshot+upload is light)."
  type        = number
  default     = 512
}

variable "task_memory" {
  description = "Fargate task memory (MiB) for the backup task. Must satisfy Fargate cpu/memory pairing rules."
  type        = number
  default     = 1024
}

variable "s3_prefix" {
  description = "Key prefix under the bucket the backup tarballs land at (S3_PREFIX in backup-cron.sh)."
  type        = string
  default     = "argo-backups"
}

variable "retention_days" {
  description = "Local-tarball prune horizon passed to backup-cron.sh (RETENTION_DAYS). Remote retention is governed by the bucket lifecycle rule below, not this."
  type        = number
  default     = 14
}

variable "schedule_expression" {
  description = "EventBridge Scheduler expression for the daily backup. Default = 17:00 UTC daily (early-morning AEST, off-peak)."
  type        = string
  default     = "cron(0 17 * * ? *)"
}

variable "schedule_timezone" {
  description = "IANA timezone the schedule_expression is evaluated in."
  type        = string
  default     = "UTC"
}

variable "scheduled_backup_enabled" {
  description = "DEC-056: whether the daily EFS-mounting backup task is ENABLED. Meaningful only when the gateway DB is on EFS — with the DB on local disk (gateway_db_on_efs=false / B′) this task backs up an empty EFS and fails daily, so the root sets it to var.gateway_db_on_efs and the in-container snapshot loop provides durability instead. The bucket + IAM + lifecycle stay regardless (B′ snapshots use the same bucket)."
  type        = bool
  default     = true
}

variable "transition_glacier_days" {
  description = "Days after which a backup object transitions to GLACIER (cold, cheap)."
  type        = number
  default     = 7
}

variable "expiration_days" {
  description = "Days after which a current backup object expires (is deleted). 35 keeps ~5 weeks of daily backups."
  type        = number
  default     = 35
}

variable "noncurrent_version_expiration_days" {
  description = "Days after which a NONCURRENT (superseded) object version is permanently deleted, so versioning does not grow unbounded."
  type        = number
  default     = 35
}
