# SKYCOT — least-privilege IAM for the offsite backup task + its scheduler.
#
# Two identities:
#   * backup_task   — the runtime role the backup container assumes. Granted
#                     ONLY s3:Put/Get/List on the backup bucket ARN (no
#                     wildcards), plus KMS data-key ops when the bucket is
#                     SSE-KMS. It never touches any other bucket or AWS API.
#   * scheduler     — the EventBridge Scheduler role. Granted ONLY ecs:RunTask
#                     on the backup task definition + iam:PassRole on the two
#                     roles that task uses, scoped to ecs-tasks.amazonaws.com.

data "aws_region" "current" {}
data "aws_caller_identity" "current" {}

locals {
  ecs_cluster_arn = "arn:aws:ecs:${data.aws_region.current.name}:${data.aws_caller_identity.current.account_id}:cluster/${var.ecs_cluster_name}"

  # The backup task definition's family ARN with a :* revision wildcard, so the
  # scheduler can RunTask across future revisions without an IAM change.
  backup_task_family_arn = replace(aws_ecs_task_definition.backup.arn, "/:[0-9]+$/", ":*")
}

# ---- backup task (runtime) role --------------------------------------------

data "aws_iam_policy_document" "ecs_assume" {
  statement {
    effect  = "Allow"
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["ecs-tasks.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "backup_task" {
  name               = "${var.name_prefix}-backup-task"
  assume_role_policy = data.aws_iam_policy_document.ecs_assume.json
}

data "aws_iam_policy_document" "backup_s3" {
  # List is a bucket-level action — scoped to the bucket ARN.
  statement {
    sid       = "ListBackupBucket"
    effect    = "Allow"
    actions   = ["s3:ListBucket"]
    resources = [aws_s3_bucket.backups.arn]
  }

  # Object writes/reads — scoped to objects UNDER this bucket only. No "*"
  # resource; no second bucket reachable.
  statement {
    sid       = "ReadWriteBackupObjects"
    effect    = "Allow"
    actions   = ["s3:PutObject", "s3:GetObject"]
    resources = ["${aws_s3_bucket.backups.arn}/*"]
  }

  # SSE-KMS uploads/downloads need data-key ops on the CMK. Only emitted when the
  # bucket is SSE-KMS; scoped to the one key ARN.
  dynamic "statement" {
    for_each = local.use_kms ? [1] : []
    content {
      sid       = "BackupBucketKms"
      effect    = "Allow"
      actions   = ["kms:GenerateDataKey", "kms:Decrypt"]
      resources = [var.kms_key_arn]
    }
  }
}

resource "aws_iam_role_policy" "backup_s3" {
  name   = "${var.name_prefix}-backup-s3"
  role   = aws_iam_role.backup_task.id
  policy = data.aws_iam_policy_document.backup_s3.json
}

# ---- EventBridge Scheduler role --------------------------------------------

data "aws_iam_policy_document" "scheduler_assume" {
  statement {
    effect  = "Allow"
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["scheduler.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "scheduler" {
  name               = "${var.name_prefix}-backup-scheduler"
  assume_role_policy = data.aws_iam_policy_document.scheduler_assume.json
}

data "aws_iam_policy_document" "scheduler" {
  statement {
    sid       = "RunBackupTask"
    effect    = "Allow"
    actions   = ["ecs:RunTask"]
    resources = [local.backup_task_family_arn]

    condition {
      test     = "ArnLike"
      variable = "ecs:cluster"
      values   = [local.ecs_cluster_arn]
    }
  }

  # PassRole only the two roles the backup task uses, and only to ECS tasks.
  statement {
    sid       = "PassBackupTaskRoles"
    effect    = "Allow"
    actions   = ["iam:PassRole"]
    resources = [var.execution_role_arn, aws_iam_role.backup_task.arn]

    condition {
      test     = "StringLike"
      variable = "iam:PassedToService"
      values   = ["ecs-tasks.amazonaws.com"]
    }
  }
}

resource "aws_iam_role_policy" "scheduler" {
  name   = "${var.name_prefix}-backup-scheduler"
  role   = aws_iam_role.scheduler.id
  policy = data.aws_iam_policy_document.scheduler.json
}
