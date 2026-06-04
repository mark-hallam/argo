output "execution_role_arn" {
  description = "ARN of the ECS task-execution role (pull image, fetch secrets, write logs)."
  value       = aws_iam_role.execution.arn
}

output "task_role_arn" {
  description = "ARN of the runtime task role. Empty today; attach policies here when the container needs AWS APIs."
  value       = aws_iam_role.task.arn
}
