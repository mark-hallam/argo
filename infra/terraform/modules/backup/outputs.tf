output "backup_bucket_name" {
  description = "Name of the offsite backup S3 bucket. Pass as S3_BUCKET to scripts/restore-drill.sh."
  value       = aws_s3_bucket.backups.id
}

output "backup_bucket_arn" {
  description = "ARN of the offsite backup S3 bucket."
  value       = aws_s3_bucket.backups.arn
}

output "backup_schedule_arn" {
  description = "ARN of the EventBridge Scheduler schedule that runs the daily backup RunTask."
  value       = aws_scheduler_schedule.backup.arn
}

output "backup_task_definition_arn" {
  description = "ARN of the backup task definition (informational; the scheduler RunTask's this)."
  value       = aws_ecs_task_definition.backup.arn
}
