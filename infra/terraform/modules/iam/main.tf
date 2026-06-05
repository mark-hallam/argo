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
# The argo binary does NOT make AWS API calls. Today this role is empty (just an
# assumable identity with no policies). Future surfaces that need AWS APIs
# (S3-backed artefacts, SES email, etc.) attach policies here.
resource "aws_iam_role" "task" {
  name               = "${var.name_prefix}-task"
  assume_role_policy = data.aws_iam_policy_document.ecs_assume.json
}
