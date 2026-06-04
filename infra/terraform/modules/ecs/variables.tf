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
