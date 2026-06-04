output "file_system_id" {
  description = "ID of the EFS filesystem."
  value       = aws_efs_file_system.this.id
}

output "access_point_id" {
  description = "ID of the EFS access point the task mounts at /data."
  value       = aws_efs_access_point.argo.id
}

output "security_group_id" {
  description = "EFS SG id (informational; not wired anywhere else)."
  value       = aws_security_group.efs.id
}
