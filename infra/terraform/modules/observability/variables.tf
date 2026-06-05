variable "name_prefix" {
  description = "Resource-name prefix, e.g. skycot-prod."
  type        = string
}

variable "alert_email" {
  description = "Optional email to subscribe to the alerts SNS topic. Empty = topic created with no subscription (operator wires actions later). The subscription needs email confirmation."
  type        = string
  default     = ""
}

variable "alb_arn_suffix" {
  description = "ALB ARN suffix (from the alb module) for CloudWatch LoadBalancer dimension."
  type        = string
}

variable "target_group_arn_suffix" {
  description = "Target-group ARN suffix (from the alb module) for CloudWatch TargetGroup dimension."
  type        = string
}

variable "cluster_name" {
  description = "ECS cluster name (from the ecs module)."
  type        = string
}

variable "service_name" {
  description = "ECS service name (from the ecs module)."
  type        = string
}

variable "target_5xx_threshold" {
  description = "Alarm when target (gateway) 5xx responses over 5 min exceed this."
  type        = number
  default     = 10
}
