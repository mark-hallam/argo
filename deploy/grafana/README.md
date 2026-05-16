# SKYCOT observability — Prometheus, Grafana, alerts

End-to-end operator setup. Files ship as code under `deploy/`:

```
deploy/
├── grafana/dashboards/{skycot-overview,provider-spend,action-dispatch,audit-chain}.json
├── grafana/README.md          # this file
├── prometheus/scrape-config.yml
└── alertmanager/rules.yml
```

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
