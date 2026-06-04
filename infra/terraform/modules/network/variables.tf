variable "name_prefix" {
  description = "Resource-name prefix, e.g. skycot-prod."
  type        = string
}

variable "vpc_cidr" {
  description = "VPC CIDR. Must be a /16 or /20 sized large enough to carve 2*az_count subnets."
  type        = string
}

variable "az_count" {
  description = "Number of AZs to span. The module places one public + one private subnet per AZ."
  type        = number
}
