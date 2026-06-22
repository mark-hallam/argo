# SKYCOT — scheduled overage-billing caller (EventBridge Scheduler -> ECS RunTask).
#
# Wave-3 ③ — THE missing piece. The overage job (`argo billing report-overage`)
# + the 10% real-cost markup shipped (#2996/#2999), but nothing CALLED the job on
# a schedule, so a paying customer's metered usage never billed unless an operator
# ran the CLI by hand. This module is that scheduled caller.
#
# A small dedicated Fargate task (reusing the gateway IMAGE + the same Stripe /
# DB / workspace secrets, but its OWN least-privilege task role from iam.tf) runs
# `bash scripts/overage-cron.sh` once a day. That wrapper restores the gateway's
# latest DB snapshot from S3 (DEC-056 B′: the live DB is on the gateway task's
# local disk, only reachable via its S3 snapshot), bridges the .argo db to the
# .skycot path the CLI expects, then runs the idempotent report-overage command.
#
# EventBridge Scheduler (not a classic CloudWatch Events rule) is used because it
# is the current first-class AWS scheduler with native ECS targets, retries, and a
# flexible time window — and because it MIRRORS modules/backup/schedule.tf so the
# two scheduled tasks share one style.
#
# IMAGE PREREQUISITE (same as the backup task): the runtime image bundles
# `scripts/` at /app/scripts and installs AWS CLI v2 (root Dockerfile +
# deploy/Dockerfile.gateway, both at parity); its ENTRYPOINT is the `argo` binary,
# overridden to /bin/bash here. scripts/overage-cron.sh ships in that same
# /app/scripts dir (no Dockerfile change needed — the whole scripts/ tree is
# COPY'd in).
#
# IDEMPOTENCY / CADENCE: `argo billing report-overage` pushes each Pro tenant's
# CUMULATIVE period-to-date overage with Stripe action=set keyed on a stable
# (tenant_id, period_start) idempotency key, so re-running RE-ASSERTS the same
# totals rather than double-billing. DAILY is the chosen cadence: it is well
# inside Stripe's monthly invoice window (a missed day is caught by the next run
# before the period closes), keeps the billed amount near-current for usage
# dashboards, and costs one short Fargate task per day. Hourly would be equally
# safe (idempotent) but is needless cost for monthly-billed metered usage.

resource "aws_cloudwatch_log_group" "overage" {
  name              = "/aws/ecs/${var.name_prefix}-overage"
  retention_in_days = var.log_retention_days
}

locals {
  # Stripe metered subscription item env: the report-overage command pushes to
  # the price named by STRIPE_PRICE_PRO_OVERAGE. UNSET => the command plans-only
  # (no Stripe push) and exits 0 — a safe default until the operator wires the
  # metered price. Dropped when empty so the binary keeps that plan-only path.
  overage_price_env = var.stripe_price_pro_overage != "" ? [
    { name = "STRIPE_PRICE_PRO_OVERAGE", value = var.stripe_price_pro_overage },
  ] : []

  overage_container = [{
    name       = "overage"
    image      = "${var.image_repo_url}:${var.image_tag}"
    essential  = true
    entryPoint = ["/bin/bash"]
    command    = ["scripts/overage-cron.sh"]

    environment = concat([
      # Where the live DB snapshots live (DEC-056 B′). Same bucket+prefix the
      # gateway snapshots to; the wrapper restores the latest before reporting.
      # Empty bucket => the wrapper reads the DB in place (EFS / dev), no restore.
      { name = "DB_SNAPSHOT_S3_BUCKET", value = var.db_snapshot_bucket },
      { name = "DB_SNAPSHOT_S3_PREFIX", value = var.db_snapshot_prefix },
      # Root the restore/.argo/.skycot bridge at /data (local disk on Fargate).
      { name = "ARGO_WORKSPACE", value = "/data" },
      { name = "ARGO_HOME", value = "/data/.argo" },
      { name = "ARGO_BIN", value = "/usr/local/bin/argo" },
      # Route any LLM-touching code path at the HTTP API, not the absent CLI
      # binary (matches the gateway task def). report-overage itself is LLM-free,
      # but this keeps the task def honest if the command ever grows one.
      { name = "ARGO_GENESIS_LLM_PROVIDER", value = "anthropic" },
      # The AWS CLI needs a region for the s3 restore (no ~/.aws/config in-container).
      { name = "AWS_DEFAULT_REGION", value = var.region },
      # Plan-only unless --json/--dry-run passed; default REPORT_OVERAGE_ARGS empty
      # = real push when STRIPE_PRICE_PRO_OVERAGE is set.
      { name = "REPORT_OVERAGE_ARGS", value = var.report_overage_args },
    ], local.overage_price_env)

    # The argo binary opens the at-rest-encrypted SQLCipher store + needs the
    # Stripe key to push usage. Sourced from Secrets Manager via the reused
    # execution role's GetSecretValue grant — never plaintext env values.
    secrets = [
      { name = "SKYCOT_DB_KEY_BASE64", valueFrom = var.secret_arns["db_key"] },
      { name = "STRIPE_SECRET_KEY", valueFrom = var.secret_arns["stripe_secret_key"] },
      # The credentials master key — the store decrypts per-tenant rows with it.
      { name = "ARGO_CREDENTIALS_MASTER_KEY", valueFrom = var.secret_arns["workspace_root_key"] },
      { name = "SKYCOT_WORKSPACE_ROOT_KEY_BASE64", valueFrom = var.secret_arns["workspace_root_key"] },
    ]

    logConfiguration = {
      logDriver = "awslogs"
      options = {
        "awslogs-group"         = aws_cloudwatch_log_group.overage.name
        "awslogs-region"        = var.region
        "awslogs-stream-prefix" = "overage"
      }
    }
  }]
}

resource "aws_ecs_task_definition" "overage" {
  family                   = "${var.name_prefix}-overage"
  cpu                      = tostring(var.task_cpu)
  memory                   = tostring(var.task_memory)
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  execution_role_arn       = var.execution_role_arn
  task_role_arn            = aws_iam_role.overage_task.arn

  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = "X86_64"
  }

  # No volume: the overage task restores the DB from S3 onto its own local disk
  # (B′), so it does NOT mount the gateway's EFS. (If a future deploy puts the DB
  # on EFS, point db_snapshot_bucket="" and add an EFS volume here to read in
  # place — left out now to keep the task minimal for the live B′ topology.)
  container_definitions = jsonencode(local.overage_container)
}

resource "aws_scheduler_schedule" "overage" {
  name        = "${var.name_prefix}-overage-daily"
  description = "Daily Pro metered-overage report: restore DB snapshot -> argo billing report-overage -> Stripe."

  # Gated behind enable_overage_billing_schedule (root default true). Disabling
  # leaves the task-def + IAM in place but stops the daily firing.
  state = var.enable_overage_billing_schedule ? "ENABLED" : "DISABLED"

  flexible_time_window {
    mode = "OFF"
  }

  schedule_expression          = var.schedule_expression
  schedule_expression_timezone = var.schedule_timezone

  target {
    arn      = local.ecs_cluster_arn
    role_arn = aws_iam_role.scheduler.arn

    ecs_parameters {
      task_definition_arn = aws_ecs_task_definition.overage.arn
      task_count          = 1
      launch_type         = "FARGATE"

      network_configuration {
        subnets          = var.private_subnets
        security_groups  = [var.service_sg_id]
        assign_public_ip = false
      }
    }

    # A failed run retries a few times within an hour, then drops — the next
    # daily run is the real recovery, and idempotency means no double-bill when
    # both eventually land.
    retry_policy {
      maximum_event_age_in_seconds = 3600
      maximum_retry_attempts       = 3
    }
  }
}
