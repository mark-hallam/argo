variable "zone_id" {
  description = "Cloudflare zone id for skycot.com."
  type        = string
}

variable "api_host" {
  description = "Public API hostname the rules scope to."
  type        = string
  default     = "api.skycot.com"
}

# Per-IP rate-limit knobs (requests per `period` seconds). These sit ABOVE the
# gateway's own per-IP limiter (config [gateway.rate_limit.ip], default cap 120 /
# refill 2/s) as defense-in-depth at the edge. Generous by default — they catch
# abuse, not normal use.

variable "auth_requests_per_minute" {
  description = "Per-IP cap on /api/v1/auth/* (signup/verify/login). Tight — these are cheap to abuse."
  type        = number
  default     = 30
}

variable "genesis_requests_per_minute" {
  description = "Per-IP cap on /api/v1/genesis/* (LLM authoring — the expensive anonymous path)."
  type        = number
  default     = 20
}

variable "api_requests_per_minute" {
  description = "Per-IP global ceiling on /api/v1/* — generous; defense-in-depth above the app limiter."
  type        = number
  default     = 600
}

variable "mitigation_timeout_secs" {
  description = "How long (seconds) a rate-limited IP stays mitigated after tripping a rule."
  type        = number
  default     = 60
}
