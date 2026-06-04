# modules/ecs

ECS Fargate cluster + task definition + service + log group.

Provisions:
- An ECS cluster with Container Insights enabled.
- A single-Fargate task definition for the `argo` container:
  - Pulls `<ecr_repo>:<image_tag>`.
  - cpu/memory from variables (defaults 1024/2048 = 1 vCPU / 2 GB).
  - env: `ARGO_GATEWAY_PROFILE=production`, `ARGO_HOME=/data`, `RUST_LOG=info`.
  - secrets (injected from Secrets Manager): `ARGO_GATEWAY_BEARER_TOKEN`,
    `SKYCOT_WORKSPACE_ROOT_KEY_BASE64`, `ARGO_CREDENTIALS_MASTER_KEY`,
    `SKYCOT_AUTH_JWT_SECRET`, `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`,
    `SKYCOT_STRIPE_PRICE_IDS`.
  - mounts the EFS access point at `/data`.
  - awslogs driver pointed at `/aws/ecs/<prefix>-argo` (retention from variable).
- A service with `desired_count=1`, Recreate-style deployment
  (`min_healthy=0`/`max=100`), wired to the ALB target group.
- A 30-day-default CloudWatch Logs group.

## Deploy semantics

- `lifecycle.ignore_changes = [task_definition, desired_count]` so the operator can roll
  out new images via `aws ecs update-service --force-new-deployment` without terraform
  fighting the deploy. To register a new task-definition revision, the operator uses the
  AWS console or a one-off `aws ecs register-task-definition` call. (Later we can add a
  CI flow that re-runs terraform with a pinned image_tag instead.)

## `ARGO_GATEWAY_PROFILE=production`

This is the critical hardening env: the gateway's `routes_auth.rs` enforces
`auth_bypass = false` when the profile is `production`. Without this env, a non-loopback
bind would refuse to start unless a bearer token is set, but `production` also disables
the dev shortcuts and is required for the WS5 hardening criteria.
