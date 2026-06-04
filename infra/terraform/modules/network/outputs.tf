output "vpc_id" {
  description = "ID of the freshly-provisioned VPC."
  value       = aws_vpc.this.id
}

output "public_subnet_ids" {
  description = "Public subnet IDs (ALB lives here)."
  value       = aws_subnet.public[*].id
}

output "private_subnet_ids" {
  description = "Private subnet IDs (Fargate tasks + EFS mount targets live here)."
  value       = aws_subnet.private[*].id
}

output "vpc_cidr" {
  description = "CIDR of the VPC, re-emitted for downstream module convenience."
  value       = aws_vpc.this.cidr_block
}
