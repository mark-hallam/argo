variable "name_prefix" {
  description = "Resource-name prefix, e.g. skycot-prod."
  type        = string
}

variable "region" {
  description = "AWS region; used for the awslogs driver config."
  type        = string
}

variable "private_subnets" {
  description = "Private subnet IDs the Fargate task ENIs live in."
  type        = list(string)
}

variable "service_sg_id" {
  description = "Security group the service tasks attach to (created in alb module)."
  type        = string
}

variable "target_group_arn" {
  description = "ALB target group the service registers with."
  type        = string
}

variable "alb_listener_arn" {
  description = "HTTPS listener ARN. Service depends on this so first-deploy doesn't race the listener."
  type        = string
}

variable "image_repo_url" {
  description = "ECR repo URL the task pulls from."
  type        = string
}

variable "image_tag" {
  description = "Container image tag."
  type        = string
}

variable "task_cpu" {
  description = "Fargate CPU units."
  type        = number
}

variable "task_memory" {
  description = "Fargate task memory, MiB."
  type        = number
}

variable "log_retention_days" {
  description = "CloudWatch Logs retention."
  type        = number
}

variable "execution_role_arn" {
  description = "Task execution role ARN."
  type        = string
}

variable "task_role_arn" {
  description = "Task (runtime) role ARN."
  type        = string
}

variable "efs_file_system_id" {
  description = "EFS filesystem id holding /data."
  type        = string
}

variable "efs_access_point_id" {
  description = "EFS access point id the task mounts."
  type        = string
}

variable "secret_arns" {
  description = "Map of secret key -> ARN. Injected as env via the task definition `secrets` block."
  type        = map(string)
}

variable "container_port" {
  description = "Container listening port."
  type        = number
  default     = 8080
}

variable "trust_xff" {
  description = "When true, inject ARGO_TRUST_XFF=1 so the gateway's per-IP limiter keys on the real client IP behind Cloudflare. Set together with the alb module's front_with_cloudflare (which makes XFF non-spoofable)."
  type        = bool
  default     = false
}

variable "public_base_url" {
  description = "SKYCOT_PUBLIC_BASE_URL — public origin the SPA is served from; magic-link + workspace-invite emails mint links against it. Empty = the gateway falls back to the hardcoded https://app.skycot.com default and logs a startup WARNING."
  type        = string
  default     = ""
}

variable "email_outbound" {
  description = "Outbound transactional-email config for magic-link delivery. provider in {noop,postmark,sendgrid,smtp}. For AWS SES use provider=smtp with host email-smtp.<region>.amazonaws.com + the SES SMTP username. The SMTP password is injected from Secrets Manager (key `smtp_password` in secret_arns) as SKYCOT_SMTP_PASSWORD, NEVER from plaintext here."
  type = object({
    provider      = optional(string, "")
    from_address  = optional(string, "")
    smtp_host     = optional(string, "")
    smtp_port     = optional(string, "")
    smtp_username = optional(string, "")
  })
  default = {}
}
