output "alb_dns_name" {
  description = "Public DNS name of the ALB."
  value       = aws_lb.this.dns_name
}

output "alb_zone_id" {
  description = "Hosted zone ID of the ALB (for Route53 alias records)."
  value       = aws_lb.this.zone_id
}

output "target_group_arn" {
  description = "Target-group ARN. ECS service registers its task IPs here."
  value       = aws_lb_target_group.this.arn
}

output "alb_arn_suffix" {
  description = "ALB ARN suffix for CloudWatch metric dimensions (LoadBalancer)."
  value       = aws_lb.this.arn_suffix
}

output "target_group_arn_suffix" {
  description = "Target-group ARN suffix for CloudWatch metric dimensions (TargetGroup)."
  value       = aws_lb_target_group.this.arn_suffix
}

output "https_listener_arn" {
  description = "HTTPS listener ARN."
  value       = aws_lb_listener.https.arn
}

output "certificate_arn" {
  description = "ACM cert ARN. Pending validation on first apply unless route53_zone_id is set."
  value       = aws_acm_certificate.this.arn
}

output "certificate_validation_records" {
  description = "DNS records the operator creates out-of-band (Cloudflare) to validate the ACM cert. Empty when route53_zone_id is set (validation is automatic)."
  value = [
    for dvo in aws_acm_certificate.this.domain_validation_options : {
      name  = dvo.resource_record_name
      type  = dvo.resource_record_type
      value = dvo.resource_record_value
    }
  ]
}

output "service_sg_id" {
  description = "Security-group id the ECS service tasks attach to."
  value       = aws_security_group.service.id
}

output "alb_sg_id" {
  description = "Security-group id the ALB attaches to (informational)."
  value       = aws_security_group.alb.id
}
