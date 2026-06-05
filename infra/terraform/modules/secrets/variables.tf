variable "name_prefix" {
  description = "Resource-name prefix, e.g. skycot-prod."
  type        = string
}

variable "kms_key_arn" {
  description = "ARN of the customer-managed CMK (from the kms module) used to encrypt every secret."
  type        = string
}
