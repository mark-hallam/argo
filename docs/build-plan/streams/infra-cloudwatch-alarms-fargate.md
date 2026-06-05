# infra-cloudwatch-alarms-fargate

**Owner**: claude (Windows lane B — backend+infra lane).
**Depends on**: WS5-1 (#2683) + WS5-3 (#2685) + #2692 — **STACKED on the #2692 branch** (adds ALB ARN-suffix outputs on top of #2692's ALB changes; rebases clean as the stack merges).
**Authored against**: origin/main `54d3f5395` on `2026-06-04`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh infra-cloudwatch-alarms-fargate`.

## Why

Observability is priority #2. The AWS IaC ships CloudWatch **Logs** but no **alarms** — so a
crash-loop, a failed deploy, or a 5xx storm on the single-task gateway would be silent until a
user complains. The existing Grafana/Prometheus dashboards (`deploy/grafana`, `deploy/prometheus`)
visualise the app's own `/openmetrics`, but nothing watches the AWS infra layer (task health,
ALB 5xx, target health) or pages anyone.

## What

A new `infra/terraform/modules/observability/`:
- An SNS topic (`<prefix>-alerts`) + optional email subscription (`alert_email` var).
- Six CloudWatch alarms → SNS (with `ok_actions` too, so recovery notifies):
  1. `ecs-running-tasks-low` — <1 running task (gateway down / crash-loop; `missing=breaching`).
  2. `alb-unhealthy-hosts` — 0 healthy targets.
  3. `alb-target-5xx` — gateway 5xx > threshold over 5 min.
  4. `alb-elb-5xx` — ALB-level 5xx (no healthy target / ALB failure).
  5. `ecs-cpu-high` — CPU >85% for 15 min (SQLite single-writer can't scale out — saturation).
  6. `ecs-memory-high` — memory >85% for 15 min.
- ALB module gains `alb_arn_suffix` + `target_group_arn_suffix` outputs (CloudWatch dimensions).
- Root wires the module + an `alert_email` var + an `alerts_sns_topic_arn` output.

## May-touch
- `infra/terraform/modules/observability/**` (NEW).
- `infra/terraform/modules/alb/outputs.tf` (add arn_suffix outputs).
- `infra/terraform/{main.tf,variables.tf,outputs.tf,terraform.tfvars.example}` (wire module).
- `docs/build-plan/BUILD_STATUS.md` (ledger flip).

Do NOT change any Rust crate.

## Acceptance criteria
- [ ] `cd infra/terraform && terraform validate` + `terraform fmt -recursive -check` exit 0.
- [ ] 6 alarms + 1 SNS topic; `alert_email` empty → topic with no subscription (no error).
- [ ] Each alarm has `alarm_actions` + `ok_actions` = the SNS topic.
- [ ] `ecs-running-tasks-low` treats missing data as breaching (a vanished task pages).
- [ ] No file >500 LOC; no Rust changed.

## Out of scope
- A live apply (operator-gated).
- Slack/PagerDuty wiring (operator subscribes the SNS topic).
- App-level metric alarms (the Grafana/Prometheus stack covers `/openmetrics`).
- Auto-scaling (single-writer SQLite is single-task by design; CPU/mem alarms are for awareness).

## Parallel-surface check
- `rg 'aws_cloudwatch_metric_alarm|modules/observability' infra/` — none before this; new.
- Complements (does not duplicate) `deploy/grafana` + `deploy/prometheus` (app metrics vs AWS infra).

## Source
Priorities: observability (#2) + reliability (#1). Deploy index `docs/skycot/deploy/README.md`.
