# SKYCOT — Cloudflare WAF + edge rate-limiting for api.skycot.com.
# Reusable child module — provisioned by the Cloudflare apply in deploy/dns
# (`module "waf"`), so the SAME `cd deploy/dns && terraform apply` that sets DNS
# also enforces the edge rate-limit by default (no separate forgotten apply).
# The provider is configured by the ROOT (deploy/dns); this module only declares
# the requirement. Auth: CLOUDFLARE_API_TOKEN (Zone:WAF + Zone:Ruleset + DNS edit).

terraform {
  required_version = ">= 1.5"
  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.0"
    }
  }
}

