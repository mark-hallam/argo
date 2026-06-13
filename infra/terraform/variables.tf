variable "region" {
  description = "AWS region to provision into. Sprint target = Sydney."
  type        = string
  default     = "ap-southeast-2"
}

variable "env" {
  description = "Environment slug used for resource naming + tags (prod, staging, dev)."
  type        = string
  default     = "prod"
}

variable "project" {
  description = "Project slug used for resource naming + tags."
  type        = string
  default     = "skycot"
}

variable "domain_name" {
  description = "Public hostname the ALB serves (TLS via ACM). DNS is managed out of band."
  type        = string
  default     = "api.skycot.com"
}

variable "vpc_cidr" {
  description = "CIDR for the freshly-provisioned VPC."
  type        = string
  default     = "10.40.0.0/16"
}

variable "az_count" {
  description = "Number of AZs to spread subnets across. 2 is the AWS-recommended minimum for an ALB."
  type        = number
  default     = 2

  validation {
    condition     = var.az_count >= 2 && var.az_count <= 3
    error_message = "az_count must be 2 or 3 (ALB requires >=2; >3 is wasted for a single-task service)."
  }
}

variable "image_tag" {
  description = "Tag of the argo container image inside the ECR repo. Operator pushes :latest by default."
  type        = string
  default     = "latest"
}

variable "task_cpu" {
  description = "Fargate CPU units (1024 = 1 vCPU). 1024 is enough for the gateway under MVP traffic."
  type        = number
  default     = 1024
}

variable "task_memory" {
  description = "Fargate task memory, MiB. Must satisfy Fargate cpu/memory pairing rules."
  type        = number
  default     = 2048
}

variable "log_retention_days" {
  description = "Retention for the CloudWatch Logs group fed by the ECS task."
  type        = number
  default     = 30
}

variable "route53_zone_id" {
  description = "Optional Route53 hosted-zone id. Set ONLY if DNS is on Route53 — the IaC then creates the ACM DNS-validation record automatically. Leave empty (default) when DNS is on Cloudflare; the operator validates the ACM cert manually per the runbook."
  type        = string
  default     = ""
}

variable "front_with_cloudflare" {
  description = "When true (the operative posture once api.skycot.com is proxied through Cloudflare): the ALB SG accepts 80/443 only from Cloudflare's IP ranges, AND the gateway gets ARGO_TRUST_XFF=1 so its per-IP rate limiter keys on the real client IP. Set false during initial bring-up (before DNS is on Cloudflare) so the ALB is directly reachable for smoke tests."
  type        = bool
  default     = false
}

variable "alert_email" {
  description = "Optional email subscribed to the CloudWatch-alarm SNS topic. Empty = topic created with no subscription (operator wires Slack/PagerDuty later)."
  type        = string
  default     = ""
}

variable "public_base_url" {
  description = "SKYCOT_PUBLIC_BASE_URL — public origin the SPA is served from; magic-link + workspace-invite emails mint links against it. Empty = the gateway falls back to the hardcoded https://app.skycot.com default and logs a startup WARNING. Threaded into module.ecs."
  type        = string
  default     = ""
}

variable "email_outbound" {
  description = "Outbound transactional-email config for magic-link delivery. provider in {noop,postmark,sendgrid,smtp}. For AWS SES use provider=smtp with host email-smtp.<region>.amazonaws.com + the SES SMTP username. The SMTP password is injected from Secrets Manager (key `smtp_password` in secret_arns) as SKYCOT_SMTP_PASSWORD, NEVER from plaintext here. Threaded into module.ecs."
  type = object({
    provider      = optional(string, "")
    from_address  = optional(string, "")
    smtp_host     = optional(string, "")
    smtp_port     = optional(string, "")
    smtp_username = optional(string, "")
  })
  default = {}
}

variable "tags" {
  description = "Extra tags merged onto every resource."
  type        = map(string)
  default     = {}
}
