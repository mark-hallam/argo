output "key_arn" {
  description = "ARN of the secrets CMK. Wired into the secrets module (kms_key_id) and the IAM decrypt grant."
  value       = aws_kms_key.secrets.arn
}

output "key_id" {
  description = "Key id of the secrets CMK."
  value       = aws_kms_key.secrets.key_id
}

output "alias_name" {
  description = "Human-friendly alias for the secrets CMK."
  value       = aws_kms_alias.secrets.name
}
