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

# Task execution role — used by the ECS agent itself: pull image from ECR,
# fetch Secrets Manager values to inject as env, write CloudWatch Logs.
resource "aws_iam_role" "execution" {
  name               = "${var.name_prefix}-task-exec"
  assume_role_policy = data.aws_iam_policy_document.ecs_assume.json
}

resource "aws_iam_role_policy_attachment" "execution_default" {
  role       = aws_iam_role.execution.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

data "aws_iam_policy_document" "secrets_read" {
  statement {
    sid       = "ReadSecrets"
    effect    = "Allow"
    actions   = ["secretsmanager:GetSecretValue"]
    resources = values(var.secret_arns)
  }

  # The secrets are encrypted with the customer-managed CMK, so reading them
  # requires kms:Decrypt on that key. Scoped to the one key ARN — no wildcard.
  statement {
    sid       = "DecryptSecretsCmk"
    effect    = "Allow"
    actions   = ["kms:Decrypt"]
    resources = [var.kms_key_arn]
  }
}

resource "aws_iam_policy" "secrets_read" {
  name   = "${var.name_prefix}-secrets-read"
  policy = data.aws_iam_policy_document.secrets_read.json
}

resource "aws_iam_role_policy_attachment" "execution_secrets" {
  role       = aws_iam_role.execution.name
  policy_arn = aws_iam_policy.secrets_read.arn
}

# Task role — used by the container at runtime.
#
# The argo binary itself makes no AWS API calls, but the DEC-056 durable-DB
# entrypoint (scripts/gateway-entrypoint.sh) does: it restores the latest DB
# snapshot from — and streams snapshots to — the offsite backup bucket.
resource "aws_iam_role" "task" {
  name               = "${var.name_prefix}-task"
  assume_role_policy = data.aws_iam_policy_document.ecs_assume.json
}

# DEC-056 ("B′"): allow the gateway task to restore/snapshot its SQLite DB to the
# offsite backup bucket. Scoped to THAT bucket only (no wildcards) + the CMK
# data-key ops its SSE-KMS encryption requires. The bucket is referenced by its
# deterministic name (modules/backup names it "${name_prefix}-backups"), so this
# attaches regardless of cross-module apply order; it is harmless when the bucket
# is absent (a no-backups apply) because the entrypoint is a no-op without
# DB_SNAPSHOT_S3_BUCKET.
data "aws_iam_policy_document" "task_db_snapshots" {
  statement {
    sid       = "ListBackupBucket"
    effect    = "Allow"
    actions   = ["s3:ListBucket"]
    resources = ["arn:aws:s3:::${var.name_prefix}-backups"]
  }
  statement {
    sid       = "ReadWriteSnapshots"
    effect    = "Allow"
    actions   = ["s3:GetObject", "s3:PutObject"]
    resources = ["arn:aws:s3:::${var.name_prefix}-backups/*"]
  }
  # SSE-KMS bucket → object crypto needs data-key ops on the same CMK. One key ARN.
  statement {
    sid       = "BackupBucketCmkDataKeys"
    effect    = "Allow"
    actions   = ["kms:GenerateDataKey", "kms:Decrypt"]
    resources = [var.kms_key_arn]
  }
}

resource "aws_iam_policy" "task_db_snapshots" {
  name   = "${var.name_prefix}-task-db-snapshots"
  policy = data.aws_iam_policy_document.task_db_snapshots.json
}

resource "aws_iam_role_policy_attachment" "task_db_snapshots" {
  role       = aws_iam_role.task.name
  policy_arn = aws_iam_policy.task_db_snapshots.arn
}
