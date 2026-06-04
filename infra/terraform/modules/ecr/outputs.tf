output "repository_url" {
  description = "Full ECR repository URL. Push images here."
  value       = aws_ecr_repository.argo.repository_url
}

output "repository_name" {
  description = "Short ECR repository name."
  value       = aws_ecr_repository.argo.name
}
