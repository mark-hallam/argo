resource "aws_cloudwatch_log_group" "this" {
  name              = "/aws/ecs/${var.name_prefix}-argo"
  retention_in_days = var.log_retention_days
}

resource "aws_ecs_cluster" "this" {
  name = "${var.name_prefix}-cluster"

  setting {
    name  = "containerInsights"
    value = "enabled"
  }
}

resource "aws_ecs_cluster_capacity_providers" "this" {
  cluster_name       = aws_ecs_cluster.this.name
  capacity_providers = ["FARGATE"]

  default_capacity_provider_strategy {
    capacity_provider = "FARGATE"
    weight            = 1
    base              = 1
  }
}

locals {
  # env var -> Secrets Manager key. Order matches the secrets module.
  # workspace_root_key is referenced TWICE because the binary accepts either env name.
  #
  # NB: stripe_price_ids is intentionally NOT injected as a container env var — the
  # gateway reads the price id from the SPA's checkout request body
  # (checkout.rs `price_id`), and the SPA sources it from the build-time public
  # `NEXT_PUBLIC_STRIPE_PRICE_*`. The `stripe_price_ids` secret is kept as an
  # operator record only (see modules/secrets), so binding it here would inject a
  # variable nothing reads. Only real gateway-read secrets are bound below.
  secret_env_bindings = [
    # Required in production: the provider startup check refuses to bind unless a
    # provider API key is set (ANTHROPIC_API_KEY or OPENAI_API_KEY). Without this
    # the gateway exits 78 (EX_CONFIG) before listening and the ALB has no target.
    { env = "ANTHROPIC_API_KEY", key = "anthropic_api_key" },
    { env = "ARGO_GATEWAY_BEARER_TOKEN", key = "bearer_token" },
    { env = "SKYCOT_WORKSPACE_ROOT_KEY_BASE64", key = "workspace_root_key" },
    { env = "ARGO_CREDENTIALS_MASTER_KEY", key = "workspace_root_key" },
    { env = "SKYCOT_AUTH_JWT_SECRET", key = "auth_jwt_secret" },
    # Required in production: the enterprise-encryption boot gate fails closed
    # unless the image is built --features bundled-sqlcipher AND this is set.
    { env = "SKYCOT_DB_KEY_BASE64", key = "db_key" },
    { env = "STRIPE_SECRET_KEY", key = "stripe_secret_key" },
    { env = "STRIPE_WEBHOOK_SECRET", key = "stripe_webhook_secret" },
    # Required in production: the audit-signing boot guard (fix-mf1) fails
    # closed unless both are set. Without them the signed audit-export
    # download + screenshot URLs fall back to a publicly-known dev secret,
    # making the links forgeable for any tenant.
    { env = "ARGO_AUDIT_EXPORT_SECRET", key = "audit_export_secret" },
    { env = "ARGO_AUDIT_SCREENSHOT_SECRET", key = "audit_screenshot_secret" },
  ]

  container_secrets = concat([
    for b in local.secret_env_bindings : {
      name      = b.env
      valueFrom = var.secret_arns[b.key]
    }
  ], local.email_secrets)

  # launch-email-delivery-honesty-and-public-base-url: the gateway reads its
  # public origin + outbound-email config from env (a production container has
  # no argo.toml). Without these, magic-link emails are log-only with wrong-host
  # links. Empty values are dropped so the gateway keeps its in-binary defaults
  # (provider=noop, https://app.skycot.com) rather than being overridden to "".
  email_env = [
    for e in [
      { name = "SKYCOT_PUBLIC_BASE_URL", value = var.public_base_url },
      { name = "SKYCOT_EMAIL_PROVIDER", value = var.email_outbound.provider },
      { name = "SKYCOT_EMAIL_FROM_ADDRESS", value = var.email_outbound.from_address },
      { name = "SKYCOT_SMTP_HOST", value = var.email_outbound.smtp_host },
      { name = "SKYCOT_SMTP_PORT", value = var.email_outbound.smtp_port },
      { name = "SKYCOT_SMTP_USERNAME", value = var.email_outbound.smtp_username },
    ] : e if e.value != ""
  ]

  # The SES/SMTP password is a SECRET → injected from Secrets Manager, never as a
  # plaintext env var. Bound only when the operator has provisioned a
  # `smtp_password` secret and passed its ARN in secret_arns (lookup default null
  # keeps existing callers that omit it valid).
  smtp_password_arn = lookup(var.secret_arns, "smtp_password", null)
  email_secrets = local.smtp_password_arn == null ? [] : [
    { name = "SKYCOT_SMTP_PASSWORD", valueFrom = local.smtp_password_arn }
  ]

  # When fronted by Cloudflare, trust the X-Forwarded-For hop so the gateway's
  # per-IP rate limiter (argo-config [gateway.rate_limit.ip], read via
  # ARGO_TRUST_XFF in loader.rs) keys on the real client IP, not Cloudflare's
  # edge IP. SAFE ONLY because the ALB SG restricts ingress to Cloudflare ranges
  # (front_with_cloudflare in the alb module) — otherwise XFF would be spoofable.
  proxy_env = var.trust_xff ? [
    { name = "ARGO_TRUST_XFF", value = "1" },
    { name = "ARGO_IP_RATE_LIMIT", value = "1" },
  ] : []

  # The gateway's CSRF/origin guard (middleware_origin_check.rs) rejects
  # state-changing requests whose `Origin` header is not in
  # ARGO_TRUSTED_ORIGINS (its unset-default list does NOT include the SPA's
  # public origin). In the split topology the browser's Origin is the SPA's
  # public origin (== public_base_url), forwarded verbatim by the Next proxy,
  # so without this every POST (onboarding/provision, billing/checkout, ...)
  # 403s `origin_rejected`. Trust the SPA origin explicitly.
  trusted_origins_env = var.public_base_url != "" ? [
    { name = "ARGO_TRUSTED_ORIGINS", value = var.public_base_url },
  ] : []

  container_env = concat([
    # ARGO_GATEWAY_PROFILE=production forces auth_bypass=false in routes_auth.rs.
    { name = "ARGO_GATEWAY_PROFILE", value = "production" },
    # ARGO_HOME=/data/.argo (NOT /data): the gateway writes its db at
    # $ARGO_HOME/sessions.db, and the daily offsite-backup RunTask
    # (modules/backup/schedule.tf) snapshots $ARGO_WORKSPACE/.argo = /data/.argo.
    # These align ONLY when the gateway ALSO writes under /data/.argo — the prior
    # /data here put the db at /data/sessions.db, so the backup's empty-db guard
    # fired LOUD on every daily run (the never-closed "ecs follow-up" the backup
    # README flagged). This task-def env OVERRIDES the image default, so it must
    # be flipped here too — the image-default change alone is a no-op on ECS.
    # Fresh-apply-safe; an existing deploy needs backup-cron.sh's one-time migration.
    { name = "ARGO_HOME", value = "/data/.argo" },
    { name = "RUST_LOG", value = "info" },
    # On-disk truth-boundary policy + pitch corpus locations. The container image
    # (root Dockerfile) bakes these same ENV defaults, so the task def is a
    # belt-and-suspenders restatement: it pins resolution to the bundled /app
    # paths even if a future image ever drops the ENV, so a policy-dependent
    # route (POST /api/v1/pitch/ask) never falls back to a missing
    # workspace_root() path (the "policy-less gateway" failure class WS5 #2763
    # deleted). Read by routes_pitch_ask.rs corpus_dir()/policy_dir().
    { name = "ARGO_PITCH_CORPUS_DIR", value = "/app/skill-library/canonical-qa/corpora/skycot" },
    { name = "ARGO_PITCH_POLICY_DIR", value = "/app/policies/truth-boundary" },
    # Route the Genesis + Pitch LLM surfaces at the Anthropic HTTP API (manifest
    # `anthropic`, keyed by the ANTHROPIC_API_KEY secret). Without this the code
    # default is `claude-code-cli` (DEC-012 dev/demo), which SPAWNS a local
    # `claude` binary — absent in the container — so the Genesis hire safety judge
    # degraded to needs_review and refused EVERY hire (task_94955825), and the
    # authenticated Pitch LLM path failed identically. Read by
    # routes_genesis_stream_llm::resolve_provider_id() (Genesis classifier +
    # safety judge) and the Pitch LLM path.
    { name = "ARGO_GENESIS_LLM_PROVIDER", value = "anthropic" },
    { name = "ARGO_PITCH_LLM_PROVIDER", value = "anthropic" },
    # DEC-056 durable DB ("B′"): the entrypoint wrapper (scripts/gateway-entrypoint.sh)
    # restores-on-boot (fail-closed) + streams whole-file snapshots of the
    # local-disk SQLite DB to S3. ARGO_WORKSPACE=/data so the snapshot
    # (backup-cron.sh) roots at /data/.argo. An EMPTY bucket makes the wrapper a
    # no-op (dev / no-backups apply) — same image runs everywhere.
    { name = "ARGO_WORKSPACE", value = "/data" },
    { name = "DB_SNAPSHOT_S3_BUCKET", value = var.db_snapshot_bucket },
    { name = "DB_SNAPSHOT_S3_PREFIX", value = var.db_snapshot_prefix },
    { name = "DB_SNAPSHOT_INTERVAL", value = var.db_snapshot_interval },
    # INCLUDE_CREDENTIALS=1: the periodic S3 backup snapshot (backup-cron.sh,
    # invoked by the gateway entrypoint) runs `argo backup create
    # --include-credentials`, so the encrypted per-tenant connector
    # credentials.toml is captured in every snapshot. Without it, connector
    # creds are NOT in the backup and are lost on task replacement (the DB is
    # restored on boot but creds are not), silently breaking every per-tenant
    # connector after a redeploy. Matches the manually-deployed prod task-def
    # :13 (image wave1-371292fb3). Read by scripts/backup-cron.sh.
    { name = "INCLUDE_CREDENTIALS", value = "1" },
  ], local.proxy_env, local.email_env, local.trusted_origins_env)

  container_def = [{
    name      = "argo"
    image     = "${var.image_repo_url}:${var.image_tag}"
    essential = true

    portMappings = [{
      containerPort = var.container_port
      protocol      = "tcp"
    }]

    environment = local.container_env
    secrets     = local.container_secrets

    # When gateway_db_on_efs=false the DB lives on the task's local disk at
    # /data/.argo (the image pre-creates + chowns /data/.argo), avoiding SQLite's
    # WAL-on-NFS incompatibility (WAL needs a -shm mmap EFS can't provide, which
    # crash-loops the gateway on /health/ready once the WAL checkpoints). Local
    # disk is ephemeral (DB resets on task restart) — fine for a bring-up smoke;
    # the durable fix is no-WAL-on-EFS (tracked follow-up).
    mountPoints = var.gateway_db_on_efs ? [{
      sourceVolume  = "argo-data"
      containerPath = "/data"
      readOnly      = false
    }] : []

    logConfiguration = {
      logDriver = "awslogs"
      options = {
        "awslogs-group"         = aws_cloudwatch_log_group.this.name
        "awslogs-region"        = var.region
        "awslogs-stream-prefix" = "argo"
      }
    }

    healthCheck = {
      command     = ["CMD-SHELL", "exit 0"]
      interval    = 30
      timeout     = 5
      retries     = 3
      startPeriod = 30
    }
  }]
}

resource "aws_ecs_task_definition" "this" {
  family                   = "${var.name_prefix}-argo"
  cpu                      = tostring(var.task_cpu)
  memory                   = tostring(var.task_memory)
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  execution_role_arn       = var.execution_role_arn
  task_role_arn            = var.task_role_arn

  runtime_platform {
    operating_system_family = "LINUX"
    cpu_architecture        = "X86_64"
  }

  dynamic "volume" {
    for_each = var.gateway_db_on_efs ? [1] : []
    content {
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
  }

  container_definitions = jsonencode(local.container_def)
}

resource "aws_ecs_service" "this" {
  name                               = "${var.name_prefix}-argo"
  cluster                            = aws_ecs_cluster.this.id
  task_definition                    = aws_ecs_task_definition.this.arn
  desired_count                      = 1
  launch_type                        = "FARGATE"
  deployment_minimum_healthy_percent = 0
  deployment_maximum_percent         = 100
  health_check_grace_period_seconds  = 60

  network_configuration {
    subnets          = var.private_subnets
    security_groups  = [var.service_sg_id]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = var.target_group_arn
    container_name   = "argo"
    container_port   = var.container_port
  }

  # Tag carries var.alb_listener_arn so terraform's implicit-dependency graph
  # orders the listener before the service. (depends_on only accepts resource
  # references, not vars; this is the canonical workaround.)
  tags = {
    Name        = "${var.name_prefix}-argo"
    ListenerArn = var.alb_listener_arn
  }

  # Operator pushes a new image + does `aws ecs update-service --force-new-deployment`.
  # Don't have terraform fight that.
  lifecycle {
    ignore_changes = [task_definition, desired_count]
  }
}
