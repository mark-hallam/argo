# SKYCOT — least-privilege IAM for the scheduled overage-billing task + its
# scheduler. Mirrors modules/backup/iam.tf (same two-identity shape).
#
# Two identities:
#   * overage_task — the runtime role the overage container assumes. It must
#                    READ the gateway's live DB snapshot from the backups bucket
#                    (DEC-056 B′: the DB is on the gateway task's local disk and
#                    only reachable via its S3 snapshot), so it gets scoped
#                    s3:GetObject/ListBucket on the backups bucket + KMS decrypt
#                    when the bucket is SSE-KMS. It NEVER writes the bucket (it
#                    only reports to Stripe), so no s3:PutObject. The Stripe key +
#                    DB key + workspace key arrive as Secrets-Manager-injected env
#                    via the reused EXECUTION role's GetSecretValue grant — this
#                    task role itself touches no other AWS API.
#   * scheduler    — the EventBridge Scheduler role. Granted ONLY ecs:RunTask on
#                    the overage task definition + iam:PassRole on the two roles
#                    that task uses, scoped to ecs-tasks.amazonaws.com.

data "aws_region" "current" {}
data "aws_caller_identity" "current" {}

locals {
  ecs_cluster_arn = "arn:aws:ecs:${data.aws_region.current.name}:${data.aws_caller_identity.current.account_id}:cluster/${var.ecs_cluster_name}"

  # The overage task definition's family ARN with a :* revision wildcard, so the
  # scheduler can RunTask across future revisions without an IAM change.
  overage_task_family_arn = replace(aws_ecs_task_definition.overage.arn, "/:[0-9]+$/", ":*")

  # The backups bucket ARN (deterministic name, same as modules/backup +
  # modules/ecs's db_snapshot_bucket). When no bucket is wired (db on EFS / dev),
  # the S3 read grant is omitted.
  read_snapshot_bucket = var.db_snapshot_bucket != ""
  snapshot_bucket_arn  = "arn:aws:s3:::${var.db_snapshot_bucket}"
}

# ---- overage task (runtime) role -------------------------------------------

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

resource "aws_iam_role" "overage_task" {
  name               = "${var.name_prefix}-overage-task"
  assume_role_policy = data.aws_iam_policy_document.ecs_assume.json
}

# Read-only access to the gateway DB snapshots in the backups bucket. Only
# emitted when a snapshot bucket is wired (B′ local-disk durability mode).
data "aws_iam_policy_document" "overage_s3_read" {
  count = local.read_snapshot_bucket ? 1 : 0

  # List is a bucket-level action — scoped to the bucket ARN.
  statement {
    sid       = "ListSnapshotBucket"
    effect    = "Allow"
    actions   = ["s3:ListBucket"]
    resources = [local.snapshot_bucket_arn]
  }

  # Object reads only — the overage task restores the latest snapshot but never
  # writes the bucket (it reports to Stripe, not to S3). No s3:PutObject.
  statement {
    sid       = "ReadSnapshotObjects"
    effect    = "Allow"
    actions   = ["s3:GetObject"]
    resources = ["${local.snapshot_bucket_arn}/*"]
  }

  # SSE-KMS objects need a data-key decrypt on the CMK. Only emitted when the
  # bucket is SSE-KMS; scoped to the one key ARN.
  dynamic "statement" {
    for_each = var.kms_key_arn != "" ? [1] : []
    content {
      sid       = "SnapshotBucketKms"
      effect    = "Allow"
      actions   = ["kms:Decrypt"]
      resources = [var.kms_key_arn]
    }
  }
}

resource "aws_iam_role_policy" "overage_s3_read" {
  count  = local.read_snapshot_bucket ? 1 : 0
  name   = "${var.name_prefix}-overage-s3-read"
  role   = aws_iam_role.overage_task.id
  policy = data.aws_iam_policy_document.overage_s3_read[0].json
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
  name               = "${var.name_prefix}-overage-scheduler"
  assume_role_policy = data.aws_iam_policy_document.scheduler_assume.json
}

data "aws_iam_policy_document" "scheduler" {
  statement {
    sid       = "RunOverageTask"
    effect    = "Allow"
    actions   = ["ecs:RunTask"]
    resources = [local.overage_task_family_arn]

    condition {
      test     = "ArnLike"
      variable = "ecs:cluster"
      values   = [local.ecs_cluster_arn]
    }
  }

  # PassRole only the two roles the overage task uses, and only to ECS tasks.
  statement {
    sid       = "PassOverageTaskRoles"
    effect    = "Allow"
    actions   = ["iam:PassRole"]
    resources = [var.execution_role_arn, aws_iam_role.overage_task.arn]

    condition {
      test     = "StringLike"
      variable = "iam:PassedToService"
      values   = ["ecs-tasks.amazonaws.com"]
    }
  }
}

resource "aws_iam_role_policy" "scheduler" {
  name   = "${var.name_prefix}-overage-scheduler"
  role   = aws_iam_role.scheduler.id
  policy = data.aws_iam_policy_document.scheduler.json
}
