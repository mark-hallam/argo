variable "name_prefix" {
  description = "Resource-name prefix, e.g. skycot-prod."
  type        = string
}

variable "region" {
  description = "AWS region; scopes the Secrets Manager ViaService grant on the key policy."
  type        = string
}
