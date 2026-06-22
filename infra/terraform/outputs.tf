output "alb_dns_name" {
  description = "Public DNS name of the ALB. Point api.skycot.com here (CNAME if behind Cloudflare; A/AAAA alias if on Route53)."
  value       = module.alb.alb_dns_name
}

output "alb_zone_id" {
  description = "Hosted zone ID of the ALB (needed for Route53 alias records)."
  value       = module.alb.alb_zone_id
}

output "acm_certificate_arn" {
  description = "ACM cert ARN for the public hostname. Pending DNS validation on first apply."
  value       = module.alb.certificate_arn
}

output "acm_validation_records" {
  description = "DNS validation records for the ACM cert. Operator creates these out-of-band (Cloudflare) unless route53_zone_id is set."
  value       = module.alb.certificate_validation_records
}

output "ecr_repository_url" {
  description = "ECR repo URL. Push the argo image here: docker push <repo>:<tag>."
  value       = module.ecr.repository_url
}

output "secret_arns" {
  description = "Map of Secrets Manager ARNs the operator populates via aws secretsmanager put-secret-value."
  value       = module.secrets.secret_arns
}

output "efs_file_system_id" {
  description = "EFS filesystem id holding /data (the SQLite store)."
  value       = module.efs.file_system_id
}

output "secrets_kms_key_arn" {
  description = "ARN of the customer-managed CMK encrypting the Secrets Manager entries."
  value       = module.kms.key_arn
}

output "ecs_cluster_name" {
  description = "Name of the ECS cluster the gateway runs in."
  value       = module.ecs.cluster_name
}

output "ecs_service_name" {
  description = "Name of the ECS service. Use for force-new-deployment on image rollout."
  value       = module.ecs.service_name
}

output "alerts_sns_topic_arn" {
  description = "SNS topic the CloudWatch alarms publish to. Subscribe Slack/PagerDuty/email."
  value       = module.observability.sns_topic_arn
}

output "backup_bucket_name" {
  description = "Name of the offsite backup S3 bucket. Pass as S3_BUCKET to scripts/backup-cron.sh + scripts/restore-drill.sh. Empty when enable_offsite_backups = false."
  value       = var.enable_offsite_backups ? module.backup[0].backup_bucket_name : ""
}

output "backup_schedule_arn" {
  description = "ARN of the EventBridge Scheduler schedule running the daily backup RunTask. Empty when enable_offsite_backups = false."
  value       = var.enable_offsite_backups ? module.backup[0].backup_schedule_arn : ""
}

output "overage_schedule_arn" {
  description = "Wave-3 ③: ARN of the EventBridge Scheduler schedule running the daily overage-billing RunTask. Empty when enable_overage_billing = false."
  value       = var.enable_overage_billing ? module.overage[0].overage_schedule_arn : ""
}

output "overage_task_definition_arn" {
  description = "Wave-3 ③: ARN of the overage task definition. Run once manually via `aws ecs run-task --cluster <cluster> --task-definition <this> --launch-type FARGATE --network-configuration ...`. Empty when enable_overage_billing = false."
  value       = var.enable_overage_billing ? module.overage[0].overage_task_definition_arn : ""
}
