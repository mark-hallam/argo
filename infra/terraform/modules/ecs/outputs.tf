output "cluster_name" {
  description = "Name of the ECS cluster."
  value       = aws_ecs_cluster.this.name
}

output "service_name" {
  description = "Name of the ECS service. Use with `aws ecs update-service --force-new-deployment` to roll a new image."
  value       = aws_ecs_service.this.name
}

output "task_definition_arn" {
  description = "Initial task-definition ARN. Service ignores updates so the operator's deploy pipeline can register new revisions independently."
  value       = aws_ecs_task_definition.this.arn
}

output "log_group_name" {
  description = "CloudWatch Logs group fed by the task."
  value       = aws_cloudwatch_log_group.this.name
}
