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

variable "front_with_cloudflare" {
  description = "When true (the secure default), the ALB SG accepts 80/443 ONLY from Cloudflare's published IP ranges (so direct-to-origin is blocked and the gateway can trust X-Forwarded-For). When false, ingress is open to the internet — only valid for the transient pre-DNS bring-up smoke (see allow_open_origin_bringup)."
  type        = bool
  default     = true
}

variable "env" {
  description = "Environment slug (prod, staging, dev). The origin-lockdown precondition only HARD-FAILS for env == \"prod\" so non-prod bring-up stays frictionless."
  type        = string
  default     = "prod"
}

variable "allow_open_origin_bringup" {
  description = "Bring-up escape hatch. When true, suppresses the prod origin-lockdown precondition so the operator can deliberately run a pre-DNS smoke against the open ALB. Default false. NOT honored by the standalone go-live gate scripts/check-alb-origin-locked.sh."
  type        = bool
  default     = false
}

variable "cloudflare_ipv4_cidrs" {
  description = "Cloudflare published IPv4 ranges (https://www.cloudflare.com/ips). Used for ALB ingress when front_with_cloudflare = true."
  type        = list(string)
  default = [
    "173.245.48.0/20",
    "103.21.244.0/22",
    "103.22.200.0/22",
    "103.31.4.0/22",
    "141.101.64.0/18",
    "108.162.192.0/18",
    "190.93.240.0/20",
    "188.114.96.0/20",
    "197.234.240.0/22",
    "198.41.128.0/17",
    "162.158.0.0/15",
    "104.16.0.0/13",
    "104.24.0.0/14",
    "172.64.0.0/13",
    "131.0.72.0/22",
  ]
}

variable "cloudflare_ipv6_cidrs" {
  description = "Cloudflare published IPv6 ranges. Used for ALB ingress when front_with_cloudflare = true."
  type        = list(string)
  default = [
    "2400:cb00::/32",
    "2606:4700::/32",
    "2803:f800::/32",
    "2405:b500::/32",
    "2405:8100::/32",
    "2a06:98c0::/29",
    "2c0f:f248::/32",
  ]
}
