variable "name_prefix" {
  description = "Resource-name prefix, e.g. skycot-prod."
  type        = string
}

variable "vpc_id" {
  description = "VPC the mount targets live in."
  type        = string
}

variable "private_subnets" {
  description = "Private subnet IDs that get a mount target."
  type        = list(string)
}

variable "service_sg_id" {
  description = "Security group of the ECS service. EFS allows NFS only from this SG."
  type        = string
}

variable "efs_access_path" {
  description = "Path inside the EFS filesystem that the access point exposes as the root for the task."
  type        = string
  default     = "/data"
}

variable "efs_uid" {
  description = "POSIX UID the access point owns the path as. Matches distroless:nonroot in the argo image."
  type        = number
  default     = 65532
}

variable "efs_gid" {
  description = "POSIX GID the access point owns the path as."
  type        = number
  default     = 65532
}
