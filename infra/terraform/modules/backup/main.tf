# SKYCOT — offsite backup bucket (S3).
#
# Independent, versioned, lifecycle'd, public-access-BLOCKED, TLS-only S3 bucket
# that the bundled `scripts/backup-cron.sh` (shipped via #2867) ships verified
# SQLite snapshots into. EFS gives filesystem-level durability, but a LOGICAL
# corruption (bad migration, accidental wipe) replicates into every EFS snapshot;
# this bucket is the independent point-in-time offsite copy. Provisioned
# durable-by-default so a fresh `terraform apply` no longer has EFS-only
# durability. Restore is proven via `scripts/restore-drill.sh`.
#
# The schedule + backup task that WRITE to this bucket live in schedule.tf; the
# scoped IAM lives in iam.tf.

locals {
  use_kms = var.kms_key_arn != ""
}

resource "aws_s3_bucket" "backups" {
  bucket = "${var.name_prefix}-backups"
}

# Versioning ON: a backup overwrite (or an attacker deleting "the" object) leaves
# the prior version recoverable. Noncurrent versions are reaped by the lifecycle
# rule below so storage does not grow unbounded.
resource "aws_s3_bucket_versioning" "backups" {
  bucket = aws_s3_bucket.backups.id

  versioning_configuration {
    status = "Enabled"
  }
}

# Hard public-access block — all four flags true. Backups are the LAST thing that
# should ever be internet-reachable; this fails closed regardless of any future
# ACL/policy mistake.
resource "aws_s3_bucket_public_access_block" "backups" {
  bucket = aws_s3_bucket.backups.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# Server-side encryption at rest. Uses the secrets CMK when one is passed (so the
# backup objects are encrypted with the same customer-managed key as the secrets
# store), else SSE-S3 (AES256). bucket_key_enabled cuts KMS request cost on
# SSE-KMS.
resource "aws_s3_bucket_server_side_encryption_configuration" "backups" {
  bucket = aws_s3_bucket.backups.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm     = local.use_kms ? "aws:kms" : "AES256"
      kms_master_key_id = local.use_kms ? var.kms_key_arn : null
    }
    bucket_key_enabled = local.use_kms
  }
}

# Lifecycle: transition to GLACIER after N days (cheap cold storage), expire the
# current version after M days, and reap noncurrent versions after K days so
# versioning does not accumulate forever. Empty `filter {}` => applies to every
# object in the bucket.
resource "aws_s3_bucket_lifecycle_configuration" "backups" {
  bucket = aws_s3_bucket.backups.id

  # Lifecycle config races bucket-versioning creation; order it after so the
  # noncurrent-version rule attaches to a versioned bucket.
  depends_on = [aws_s3_bucket_versioning.backups]

  rule {
    id     = "offsite-retention"
    status = "Enabled"

    filter {}

    transition {
      days          = var.transition_glacier_days
      storage_class = "GLACIER"
    }

    expiration {
      days = var.expiration_days
    }

    noncurrent_version_expiration {
      noncurrent_days = var.noncurrent_version_expiration_days
    }
  }
}

# Deny any non-TLS (plaintext HTTP) access to the bucket or its objects. Belt to
# the public-access-block's suspenders: even an authenticated caller cannot read
# or write a backup over an unencrypted connection.
data "aws_iam_policy_document" "tls_only" {
  statement {
    sid    = "DenyInsecureTransport"
    effect = "Deny"

    principals {
      type        = "*"
      identifiers = ["*"]
    }

    actions = ["s3:*"]

    resources = [
      aws_s3_bucket.backups.arn,
      "${aws_s3_bucket.backups.arn}/*",
    ]

    condition {
      test     = "Bool"
      variable = "aws:SecureTransport"
      values   = ["false"]
    }
  }
}

resource "aws_s3_bucket_policy" "backups" {
  bucket = aws_s3_bucket.backups.id
  policy = data.aws_iam_policy_document.tls_only.json

  # Attaching a policy to a bucket that is not yet public-access-blocked can trip
  # BlockPublicPolicy; order the block first.
  depends_on = [aws_s3_bucket_public_access_block.backups]
}
