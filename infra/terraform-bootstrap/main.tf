# SKYCOT — Terraform remote-state bootstrap.
#
# Creates the S3 bucket + DynamoDB lock table that the MAIN stack
# (infra/terraform/) uses as its `backend "s3"`. This is the canonical
# chicken-and-egg bootstrap: apply this ONCE with LOCAL state, then point the
# main stack's backend at the outputs here.
#
# Why: local state on one laptop is fragile for a production deploy — state loss
# means orphaned/duplicate AWS resources (cost leak on the $800 credit), and
# there's no apply-locking against concurrent runs. S3 (versioned, encrypted) +
# DynamoDB locking is the standard production posture. Cost ≈ pennies/month.
#
# See README.md for the exact operator command sequence.

terraform {
  required_version = ">= 1.5"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.50"
    }
  }
}

provider "aws" {
  region = var.region

  default_tags {
    tags = {
      Project   = var.project
      Env       = var.env
      ManagedBy = "terraform"
      Component = "tf-state-bootstrap"
    }
  }
}

variable "region" {
  description = "AWS region. Keep the state bucket in the same region as the deploy."
  type        = string
  default     = "ap-southeast-2"
}

variable "project" {
  type    = string
  default = "skycot"
}

variable "env" {
  type    = string
  default = "prod"
}

variable "state_bucket_name" {
  description = "Globally-unique S3 bucket name for terraform state. Override if the default collides."
  type        = string
  default     = "skycot-prod-tfstate"
}

resource "aws_s3_bucket" "state" {
  bucket = var.state_bucket_name

  # State is irreplaceable — refuse an accidental `terraform destroy` of it.
  lifecycle {
    prevent_destroy = true
  }
}

resource "aws_s3_bucket_versioning" "state" {
  bucket = aws_s3_bucket.state.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "state" {
  bucket = aws_s3_bucket.state.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "state" {
  bucket                  = aws_s3_bucket.state.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_dynamodb_table" "locks" {
  name         = "${var.project}-${var.env}-tflock"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "LockID"

  attribute {
    name = "LockID"
    type = "S"
  }
}

output "state_bucket" {
  description = "S3 bucket name for the main stack's backend \"s3\" block."
  value       = aws_s3_bucket.state.id
}

output "lock_table" {
  description = "DynamoDB table name for the main stack's backend \"s3\" dynamodb_table."
  value       = aws_dynamodb_table.locks.name
}
