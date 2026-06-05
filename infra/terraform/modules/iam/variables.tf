variable "name_prefix" {
  description = "Resource-name prefix, e.g. skycot-prod."
  type        = string
}

variable "secret_arns" {
  description = "Map of secret key -> ARN. The execution role gets secretsmanager:GetSecretValue on these."
  type        = map(string)
}

variable "kms_key_arn" {
  description = "ARN of the secrets CMK. The execution role gets kms:Decrypt scoped to this key so it can read CMK-encrypted secrets."
  type        = string
}
