resource "aws_security_group" "efs" {
  name        = "${var.name_prefix}-efs-sg"
  description = "EFS — NFS 2049 from the ECS service SG only."
  vpc_id      = var.vpc_id

  ingress {
    description     = "NFS from service tasks"
    from_port       = 2049
    to_port         = 2049
    protocol        = "tcp"
    security_groups = [var.service_sg_id]
  }

  egress {
    description = "Default ENI egress (EFS does not initiate outbound)"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${var.name_prefix}-efs-sg"
  }
}

resource "aws_efs_file_system" "this" {
  creation_token   = "${var.name_prefix}-argo-data"
  encrypted        = true
  performance_mode = "generalPurpose"
  throughput_mode  = "bursting"

  lifecycle_policy {
    transition_to_ia = "AFTER_30_DAYS"
  }

  tags = {
    Name = "${var.name_prefix}-argo-data"
  }
}

resource "aws_efs_backup_policy" "this" {
  file_system_id = aws_efs_file_system.this.id

  backup_policy {
    status = "ENABLED"
  }
}

resource "aws_efs_mount_target" "this" {
  count           = length(var.private_subnets)
  file_system_id  = aws_efs_file_system.this.id
  subnet_id       = var.private_subnets[count.index]
  security_groups = [aws_security_group.efs.id]
}

resource "aws_efs_access_point" "argo" {
  file_system_id = aws_efs_file_system.this.id

  posix_user {
    uid = var.efs_uid
    gid = var.efs_gid
  }

  root_directory {
    path = var.efs_access_path

    creation_info {
      owner_uid   = var.efs_uid
      owner_gid   = var.efs_gid
      permissions = "0750"
    }
  }

  tags = {
    Name = "${var.name_prefix}-argo-ap"
  }
}
