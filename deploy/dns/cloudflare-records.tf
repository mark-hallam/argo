# SKYCOT — Cloudflare DNS records for *.skycot.com
# Apply: terraform init && terraform apply
# Auth:  CLOUDFLARE_API_TOKEN env var (Zone:DNS edit, skycot.com).
# See deploy/dns/README.md for the operator runbook.

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

variable "zone_id" { type = string }
variable "fly_ipv4" { type = string }
variable "fly_ipv6" { type = string }
variable "ttl" {
  type    = number
  default = 300
}

# Vercel apex IPs documented at vercel.com/docs/concepts/projects/domains.
locals {
  vercel_a    = "76.76.21.21"
  vercel_aaaa = "2606:4700:90:0:f22e:fbec:5bed:a9b9"
  vercel_cn   = "cname.vercel-dns.com"

  records = {
    apex_a    = { name = "@",   type = "A",     content = local.vercel_a    }
    apex_aaaa = { name = "@",   type = "AAAA",  content = local.vercel_aaaa }
    www       = { name = "www", type = "CNAME", content = local.vercel_cn   }
    app       = { name = "app", type = "CNAME", content = local.vercel_cn   }
    api_a     = { name = "api", type = "A",     content = var.fly_ipv4      }
    api_aaaa  = { name = "api", type = "AAAA",  content = var.fly_ipv6      }
    try_a     = { name = "try", type = "A",     content = var.fly_ipv4      }
    try_aaaa  = { name = "try", type = "AAAA",  content = var.fly_ipv6      }
  }
}

resource "cloudflare_record" "skycot" {
  for_each = local.records
  zone_id  = var.zone_id
  name     = each.value.name
  type     = each.value.type
  content  = each.value.content
  ttl      = var.ttl
  proxied  = true
}
