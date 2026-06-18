# SKYCOT — daily offsite-backup schedule (EventBridge Scheduler -> ECS RunTask).
#
# A small dedicated Fargate task (reusing the gateway IMAGE + EFS access point,
# but its OWN least-privilege task role from iam.tf) runs the bundled
# `scripts/backup-cron.sh` once a day: snapshot -> verify -> `aws s3 cp` to the
# bucket in main.tf -> prune. EventBridge Scheduler (not a classic CloudWatch
# Events rule) is used because it is the current first-class AWS scheduler with
# native ECS targets, retries, and a flexible time window.
#
# ✅ IMAGE PREREQUISITE (CLOSED 2026-06-15 — fix-backup-image-bake-scripts-awscli-and-argo-home):
#   The task runs `bash scripts/backup-cron.sh` against the gateway image and
#   uploads with the AWS CLI. The runtime image now bundles `scripts/` and
#   installs AWS CLI v2 (root Dockerfile + deploy/Dockerfile.gateway, both at
#   parity); its ENTRYPOINT is the `argo` binary (overridden to /bin/bash here).
#   The bucket + lifecycle + public-access-block + scoped IAM in this module are
#   live-on-apply, and the operator can also run `scripts/backup-cron.sh` from any
#   EFS-mounting host (runbook §13b, compose/systemd path). See README.md.
#
# ✅ ARGO_HOME ALIGNMENT (CLOSED same brief): backup-cron.sh snapshots
#   `$ARGO_WORKSPACE/.argo`, so it finds the db ONLY when the gateway writes the
#   db there. The gateway now runs with ARGO_HOME=/data/.argo (runbook §13b
#   convention) — set both as the image default AND on the ECS gateway service
#   task def (modules/ecs/main.tf, which OVERRIDES the image default). The
#   empty-db guard in backup-cron.sh fails loud rather than shipping a silent
#   empty backup, so a misalignment is caught, not hidden. ⚠️ EXISTING deploys
#   need backup-cron.sh's one-time migration (fresh applies are safe).

resource "aws_cloudwatch_log_group" "backup" {
  name              = "/aws/ecs/${var.name_prefix}-backup"
  retention_in_days = var.log_retention_days
}

locals {
  backup_container = [{
    name       = "backup"
    image      = "${var.image_repo_url}:${var.image_tag}"
    essential  = true
    entryPoint = ["/bin/bash"]
    command    = ["scripts/backup-cron.sh"]

    environment = [
      { name = "S3_BUCKET", value = aws_s3_bucket.backups.id },
      { name = "S3_PREFIX", value = var.s3_prefix },
      # Snapshot the live store on the shared EFS volume.
      { name = "ARGO_WORKSPACE", value = "/data" },
      { name = "ARGO_HOME", value = "/data/.argo" },
      { name = "ARGO_BIN", value = "/usr/local/bin/argo" },
      { name = "RETENTION_DAYS", value = tostring(var.retention_days) },
      # The AWS CLI needs a region for `aws s3 cp` (no ~/.aws/config in-container).
      { name = "AWS_DEFAULT_REGION", value = var.region },
    ]

    # The argo binary opens the at-rest-encrypted SQLCipher store; inject the
    # SAME key the gateway uses. Sourced from Secrets Manager via the reused
    # execution role's GetSecretValue grant — never a plaintext env value.
    secrets = [
      { name = "SKYCOT_DB_KEY_BASE64", valueFrom = var.secret_arns["db_key"] },
    ]

    mountPoints = [{
      sourceVolume  = "argo-data"
      containerPath = "/data"
      readOnly      = false
    }]

    logConfiguration = {
      logDriver = "awslogs"
      options = {
        "awslogs-group"         = aws_cloudwatch_log_group.backup.name
        "awslogs-region"        = var.region
        "awslogs-stream-prefix" = "backup"
      }
    }
  }]
}

resource "aws_ecs_task_definition" "backup" {
  family                   = "${var.name_prefix}-backup"
  cpu                      = tostring(var.task_cpu)
  memory                   = tostring(var.task_memory)
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  execution_role_arn       = var.execution_role_arn
  task_role_arn            = aws_iam_role.backup_task.arn

  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = "X86_64"
  }

  volume {
    name = "argo-data"

    efs_volume_configuration {
      file_system_id     = var.efs_file_system_id
      transit_encryption = "ENABLED"

      authorization_config {
        access_point_id = var.efs_access_point_id
        iam             = "DISABLED"
      }
    }
  }

  container_definitions = jsonencode(local.backup_container)
}

resource "aws_scheduler_schedule" "backup" {
  name        = "${var.name_prefix}-backup-daily"
  description = "Daily offsite SQLite backup: snapshot -> verify -> S3 -> prune."

  # DEC-056: this task mounts the gateway's EFS access point to snapshot the DB,
  # so it is only meaningful when the DB is actually ON EFS. With the DB on local
  # disk (gateway_db_on_efs=false / B′), it would mount an empty EFS and fail the
  # empty-db guard every day — so it is DISABLED and the in-container snapshot
  # loop (scripts/gateway-entrypoint.sh) provides durability instead.
  state = var.scheduled_backup_enabled ? "ENABLED" : "DISABLED"

  flexible_time_window {
    mode = "OFF"
  }

  schedule_expression          = var.schedule_expression
  schedule_expression_timezone = var.schedule_timezone

  target {
    arn      = local.ecs_cluster_arn
    role_arn = aws_iam_role.scheduler.arn

    ecs_parameters {
      task_definition_arn = aws_ecs_task_definition.backup.arn
      task_count          = 1
      launch_type         = "FARGATE"

      network_configuration {
        subnets          = var.private_subnets
        security_groups  = [var.service_sg_id]
        assign_public_ip = false
      }
    }

    # A failed run retries a few times within an hour, then drops (the next
    # daily run is the real recovery — no point queueing stale snapshots).
    retry_policy {
      maximum_event_age_in_seconds = 3600
      maximum_retry_attempts       = 3
    }
  }
}
