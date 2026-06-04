variable "name_prefix" {
  description = "Resource-name prefix, e.g. skycot-prod."
  type        = string
}

variable "secret_arns" {
  description = "Map of secret key -> ARN. The execution role gets secretsmanager:GetSecretValue on these."
  type        = map(string)
}
