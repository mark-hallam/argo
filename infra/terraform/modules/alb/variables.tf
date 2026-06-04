variable "name_prefix" {
  description = "Resource-name prefix, e.g. skycot-prod."
  type        = string
}

variable "vpc_id" {
  description = "VPC the ALB + target group + SGs live in."
  type        = string
}

variable "public_subnets" {
  description = "Public subnet IDs the ALB attaches to (>=2 required by ALB)."
  type        = list(string)
}

variable "domain_name" {
  description = "Hostname the ALB serves over TLS, e.g. api.skycot.com."
  type        = string
}

variable "route53_zone_id" {
  description = "Optional Route53 hosted zone for automatic ACM DNS validation. Empty = operator validates manually (Cloudflare path)."
  type        = string
  default     = ""
}

variable "container_port" {
  description = "Port the target group forwards to. Matches the gateway's --port 8080."
  type        = number
  default     = 8080
}

variable "health_check_path" {
  description = "Health check path on the container. /health/ready matches the k8s readiness probe."
  type        = string
  default     = "/health/ready"
}
