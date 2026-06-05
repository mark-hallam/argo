# Edge defense-in-depth for the public API. These rules sit ABOVE the gateway's
# own per-IP / per-tenant / per-route limiter (argo-config sections_rate_limit)
# so abusive anonymous traffic is shed at Cloudflare before it ever reaches the
# single Fargate task. Portable to the Cloudflare Free plan (rate-limit +
# custom-firewall phases; no paid managed-ruleset entitlement assumed).

locals {
  host_match = "http.host eq \"${var.api_host}\""
}

# --- Per-IP rate limiting (phase: http_ratelimit) ---
resource "cloudflare_ruleset" "rate_limit" {
  zone_id     = var.zone_id
  name        = "skycot-api-rate-limits"
  description = "Per-IP edge rate limits for the SKYCOT public API."
  kind        = "zone"
  phase       = "http_ratelimit"

  # Auth endpoints — cheap to abuse (signup/verify/login). Challenge on excess.
  rules {
    ref         = "rl_auth"
    description = "Per-IP cap on /api/v1/auth/*"
    expression  = "(${local.host_match} and starts_with(http.request.uri.path, \"/api/v1/auth/\"))"
    action      = "managed_challenge"

    ratelimit {
      characteristics     = ["ip.src"]
      period              = 60
      requests_per_period = var.auth_requests_per_minute
      mitigation_timeout  = var.mitigation_timeout_secs
    }
  }

  # Genesis — the expensive anonymous LLM-authoring path. Hard-block on excess
  # (the strongest edge mitigation for runaway LLM cost from a single IP).
  rules {
    ref         = "rl_genesis"
    description = "Per-IP cap on /api/v1/genesis/* (LLM cost control)"
    expression  = "(${local.host_match} and starts_with(http.request.uri.path, \"/api/v1/genesis/\"))"
    action      = "block"

    ratelimit {
      characteristics     = ["ip.src"]
      period              = 60
      requests_per_period = var.genesis_requests_per_minute
      mitigation_timeout  = var.mitigation_timeout_secs
    }
  }

  # Global API ceiling — generous; defense-in-depth above the app limiter.
  rules {
    ref         = "rl_api_global"
    description = "Per-IP global ceiling on /api/v1/*"
    expression  = "(${local.host_match} and starts_with(http.request.uri.path, \"/api/v1/\"))"
    action      = "block"

    ratelimit {
      characteristics     = ["ip.src"]
      period              = 60
      requests_per_period = var.api_requests_per_minute
      mitigation_timeout  = var.mitigation_timeout_secs
    }
  }
}

# --- Custom firewall (phase: http_request_firewall_custom) ---
resource "cloudflare_ruleset" "firewall_custom" {
  zone_id     = var.zone_id
  name        = "skycot-api-custom-firewall"
  description = "Baseline custom firewall rules for the SKYCOT public API."
  kind        = "zone"
  phase       = "http_request_firewall_custom"

  # Empty User-Agent on auth is a strong scripted-abuse signal — challenge it.
  rules {
    ref         = "fw_challenge_no_ua_auth"
    description = "Managed-challenge auth requests with empty User-Agent"
    expression  = "(${local.host_match} and starts_with(http.request.uri.path, \"/api/v1/auth/\") and http.user_agent eq \"\")"
    action      = "managed_challenge"
  }
}
