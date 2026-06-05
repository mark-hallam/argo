# modules/observability

CloudWatch alarms for the single-task Fargate gateway, publishing to an SNS topic.

Provisions:
- An `aws_sns_topic` (`<prefix>-alerts`) + an optional email subscription (`alert_email`,
  needs confirmation). Operator can wire Slack/PagerDuty to the topic instead.
- Six `aws_cloudwatch_metric_alarm`s:
  | Alarm | Fires when |
  |---|---|
  | `ecs-running-tasks-low` | <1 running task for 2 min (gateway down / crash-loop). `missing=breaching`. |
  | `alb-unhealthy-hosts` | 0 healthy targets for 2 min (failing health checks). |
  | `alb-target-5xx` | gateway returns >`target_5xx_threshold` (default 10) 5xx over 5 min. |
  | `alb-elb-5xx` | ALB itself returns >5 5xx over 5 min (no healthy target / ALB failure). |
  | `ecs-cpu-high` | task CPU >85% for 15 min (SQLite single-writer can't scale out — watch saturation). |
  | `ecs-memory-high` | task memory >85% for 15 min. |

Complements the Grafana/Prometheus dashboards in `deploy/grafana` + `deploy/prometheus`
(those visualise the app's own `/openmetrics`; these alarms watch the AWS infra layer).

Each alarm sets both `alarm_actions` and `ok_actions` to the SNS topic, so the operator gets
the recovery notification too.
