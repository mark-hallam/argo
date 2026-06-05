# SKYCOT — Cloudflare WAF + edge rate-limiting for api.skycot.com.
# Standalone module (mirrors deploy/dns/). Apply separately:
#   cd deploy/waf && terraform init && terraform apply
# Auth: CLOUDFLARE_API_TOKEN env var (Zone:WAF + Zone:Ruleset edit on skycot.com).

terraform {
  required_version = ">= 1.5"
  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
  }
}

provider "cloudflare" {}
