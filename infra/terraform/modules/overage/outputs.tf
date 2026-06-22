output "overage_schedule_arn" {
  description = "ARN of the EventBridge Scheduler schedule that runs the daily overage-billing RunTask."
  value       = aws_scheduler_schedule.overage.arn
}

output "overage_task_definition_arn" {
  description = "ARN of the overage task definition (the scheduler RunTask's this; also the task to run once manually via `aws ecs run-task`)."
  value       = aws_ecs_task_definition.overage.arn
}

output "overage_task_role_arn" {
  description = "ARN of the overage task's runtime IAM role (read-only S3 snapshot access + KMS decrypt)."
  value       = aws_iam_role.overage_task.arn
}
