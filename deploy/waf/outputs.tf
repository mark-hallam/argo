output "rate_limit_ruleset_id" {
  description = "Id of the per-IP rate-limit ruleset."
  value       = cloudflare_ruleset.rate_limit.id
}

output "firewall_ruleset_id" {
  description = "Id of the custom firewall ruleset."
  value       = cloudflare_ruleset.firewall_custom.id
}
