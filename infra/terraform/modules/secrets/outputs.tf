output "secret_arns" {
  description = "Map of logical secret key -> SecretsManager ARN. Used by IAM (read grant) and ECS (env injection)."
  value       = { for k, v in aws_secretsmanager_secret.this : k => v.arn }
}

output "secret_ids" {
  description = "Map of logical secret key -> SecretsManager name. Operator uses these with `aws secretsmanager put-secret-value`."
  value       = { for k, v in aws_secretsmanager_secret.this : k => v.name }
}
