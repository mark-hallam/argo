# Customer-managed KMS key (CMK) that encrypts the production Secrets Manager
# entries. Using a CMK (rather than the AWS-managed `aws/secretsmanager` key)
# gives scoped decrypt grants, key rotation, and CloudTrail visibility on every
# secret access — the enterprise-tier hardening posture.
#
# Decrypt by the task-execution role is granted in the IAM module (kms:Decrypt
# on this key ARN); the account-root statement below delegates that authority
# to IAM policies, which is the AWS-documented pattern for a CMK.

data "aws_caller_identity" "current" {}

data "aws_iam_policy_document" "key" {
  # Account root keeps full admin so the operator is never locked out of the key
  # and IAM policies in the account can grant use of it.
  statement {
    sid       = "AccountRootFullAdmin"
    effect    = "Allow"
    actions   = ["kms:*"]
    resources = ["*"]

    principals {
      type        = "AWS"
      identifiers = ["arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"]
    }
  }

  # Secrets Manager in this region may use the key on behalf of authorized
  # callers. The ViaService condition pins usage to Secrets Manager so the key
  # can't be wielded for arbitrary crypto by another service.
  statement {
    sid    = "AllowSecretsManagerViaService"
    effect = "Allow"

    actions = [
      "kms:Decrypt",
      "kms:Encrypt",
      "kms:GenerateDataKey",
      "kms:CreateGrant",
      "kms:DescribeKey",
    ]

    resources = ["*"]

    principals {
      type        = "AWS"
      identifiers = ["arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"]
    }

    condition {
      test     = "StringEquals"
      variable = "kms:ViaService"
      values   = ["secretsmanager.${var.region}.amazonaws.com"]
    }
  }
}

resource "aws_kms_key" "secrets" {
  description             = "${var.name_prefix} — Secrets Manager CMK"
  deletion_window_in_days = 7
  enable_key_rotation     = true
  policy                  = data.aws_iam_policy_document.key.json
}

resource "aws_kms_alias" "secrets" {
  name          = "alias/${var.name_prefix}-secrets"
  target_key_id = aws_kms_key.secrets.key_id
}
