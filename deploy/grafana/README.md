# SKYCOT observability — Prometheus, Grafana, alerts

End-to-end operator setup. Files ship as code under `deploy/`:

```
deploy/
├── grafana/dashboards/{skycot-overview,provider-spend,action-dispatch,audit-chain}.json
├── grafana/provisioning/{datasources,dashboards}/*.yml  # auto-load (compose stack)
├── grafana/README.md          # this file
├── prometheus/prometheus.yml      # compose-stack scrape (host loopback :7400)
├── prometheus/scrape-config.yml   # generic copy-to-/etc template (gateway:8080)
└── alertmanager/{rules.yml, alertmanager.tmpl.yml}      # alert rules + receiver
```

## Quick start — the compose stack (recommended)

The gated-demo compose stack runs the whole monitoring plane behind one
profile flag. From the repo root, with `deploy/.env.deploy` filled in:

```bash
# 1. add SES creds + a pager inbox to deploy/.env.deploy (see .env.deploy.example):
#      SKYCOT_SMTP_HOST / _PORT / _USERNAME / _PASSWORD
#      SKYCOT_EMAIL_FROM_ADDRESS   (SES-verified sender)
#      SKYCOT_ALERT_EMAIL_TO       (inbox that receives pages)
# 2. bring up prometheus + alertmanager + grafana alongside the demo:
docker compose -f deploy/docker-compose.yml --env-file deploy/.env.deploy \
  --profile monitoring up -d
# 3. reach the UIs over an SSH tunnel (they bind 127.0.0.1 only):
#      ssh -L 9090:127.0.0.1:9090 -L 9093:127.0.0.1:9093 -L 3001:127.0.0.1:3001 <host>
#    then open http://localhost:9090 (Prometheus), :9093 (Alertmanager),
#    :3001 (Grafana — dashboards auto-provisioned, datasource pre-wired).
```

Config wiring used by the profile:
- `prometheus/prometheus.yml` — compose-stack scrape config (targets the
  gateway on the host loopback `127.0.0.1:7400`; `scrape-config.yml` below is
  the GENERIC `gateway:8080` template for a non-host-networked deploy).
- `alertmanager/alertmanager.tmpl.yml` — rendered from the SES env vars at
  container start (keeps the SMTP password out of any committed file).
- `grafana/provisioning/` — datasource + dashboard-folder providers so the 4
  dashboards load with zero clicks.

Validated in `deploy/smoke/run.sh` (`validate_monitoring_stack`): the profile
parses (`docker compose --profile monitoring config -q`), the rules + config
pass `promtool check`, and the rendered Alertmanager config passes
`amtool check-config`.

### Dashboard queries validated against live metric labels (2026-06-13)

Every query in the 4 dashboards was cross-checked against the metric families
registered in `crates/argo-telemetry/src/prometheus_registry.rs` and
`crates/argo-cli/src/handlers/gateway/routes_openmetrics.rs` — **no label
drift**:

| Dashboard          | Metric{labels} used                                            | matches code |
| ------------------ | -------------------------------------------------------------- | ------------ |
| skycot-overview    | `gateway_request_total{status}`, `gateway_request_duration_seconds_bucket`, `tenant_active_count`, `tenant_signup_total`, `workflow_cron_tick_total{name,status}` | ✅ |
| provider-spend     | `provider_cost_micros_total{tenant_id}`, `provider_call_total{provider,status}`, `provider_call_duration_seconds_bucket{provider}` | ✅ |
| action-dispatch    | `action_dispatch_total{kind,status}`, `action_dispatch_duration_seconds_bucket{kind}` | ✅ |
| audit-chain        | `argo_audit_chain_ok`, `argo_audit_chain_breaks`, `audit_chain_verify_total{result}`, `audit_rows_written_total` | ✅ |

To re-run the check after a metric rename, grep the family names above against
the two source files; a missing family means a panel will read "No data".

## Scrape paths

| Path           | Contents                                                         |
| -------------- | ---------------------------------------------------------------- |
| `/metrics`     | Store-derived gauges + registry production families (recommended). |
| `/openmetrics` | Registry-only view (gateway/rate-limit/provider).                |

Production metric families (added 2026-05-16):
`gateway_request_{total,duration_seconds}`,
`gateway_rate_limit_throttle_total`,
`action_dispatch_{total,duration_seconds}`,
`provider_{call_total,call_duration_seconds,cost_micros_total,breaker_state}`,
`tenant_{active_count,signup_total}`,
`audit_{rows_written_total,chain_verify_total,chain_tail_seq}`,
`workflow_cron_tick_total`.

## Prometheus

1. Copy `prometheus/scrape-config.yml` → `/etc/prometheus/prometheus.yml`.
   Replace `gateway:8080` with the real host:port; switch `scheme: https`
   for a TLS endpoint.
2. Copy `alertmanager/rules.yml` → `/etc/prometheus/rules/alerts.yml`.
3. Validate: `promtool check config …/prometheus.yml` and
   `promtool check rules …/alerts.yml`.
4. Reload (`SIGHUP` or `/-/reload`).

Grafana Cloud free tier works equivalently — point a Grafana-Agent or
hosted scrape at the same endpoints.

## Grafana dashboards

Import each JSON in `grafana/dashboards/` via UI → Dashboards → New →
Import. Pick the Prometheus datasource bound to the SKYCOT scrape job;
dashboards reference it via a `${DS_PROM}` variable so the import is
datasource-agnostic.

## Alertmanager routing

Rules tag alerts `severity=page` (PagerDuty) or `severity=slack`. Wire
your Alertmanager `routes` to match:

```yaml
route:
  receiver: slack-default
  routes:
    - matchers: [severity="page"]
      receiver: pagerduty
    - matchers: [severity="slack"]
      receiver: slack-default
```

PagerDuty is optional for v1; without a PD receiver, `page` alerts
fall through to the default. Receiver wiring is out of scope here.
