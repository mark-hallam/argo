# modules/alb

ALB + target group + ACM TLS cert + security groups.

Provisions:
- An internet-facing ALB in the public subnets with deletion-protection ON.
- An IP-target-type target group on the container port (default 8080), health-checked
  via `/health/ready` (matches the k8s readiness probe).
- An ACM certificate for `var.domain_name` (DNS-validated).
- HTTPS:443 listener forwarding to the target group, using TLS-1.3 minimum
  (`ELBSecurityPolicy-TLS13-1-2-2021-06`).
- HTTP:80 listener that 301-redirects to HTTPS.
- ALB security group and service security group (ingress on the container port only
  from the ALB SG).
  - **`front_with_cloudflare = false` (default):** ALB ingress 80+443 from `0.0.0.0/0` + `::/0`.
  - **`front_with_cloudflare = true`:** ALB ingress 80+443 from Cloudflare's published IP
    ranges only (`cloudflare_ipv4_cidrs` / `cloudflare_ipv6_cidrs`). Blocks direct-to-origin
    hits AND makes `X-Forwarded-For` trustworthy so the gateway's per-IP limiter
    (`ARGO_TRUST_XFF`, set by the ecs module) keys on the real client IP. Flip this on after
    `api.skycot.com` is proxied through Cloudflare (runbook §7a).

## ACM validation

- If `route53_zone_id` is set, the IaC creates the DNS validation CNAME in Route53 and waits
  for the cert to be issued.
- If not (the Cloudflare DNS path the sprint defaults to), the validation records are
  emitted as the `certificate_validation_records` output and the operator pastes them into
  Cloudflare per the runbook.
