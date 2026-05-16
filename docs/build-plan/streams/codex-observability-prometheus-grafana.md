# codex-observability-prometheus-grafana — Production metrics + dashboards + alerts (Wave 4 P1)

**Owner**: codex. **Depends on**: `codex-ci-workflows-reactivate` MERGED (✅ #1732) + `codex-hosted-deployment-flyio` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1).
- **Production readiness for viral surge** — without observability under load, can't diagnose failures.

## Scope

P1. Prometheus exporter inside argo gateway + Grafana dashboards + critical alerts.

Metrics to expose at `/metrics`:
- Request: count, latency (p50, p95, p99), error rate.
- Action: dispatch count by kind, success rate, latency.
- Provider: call count, token spend, latency.
- Tenant: active count, signups/hour.
- Audit: rows/sec written, chain-verify pass rate.
- Workflow: cron tick count, failure rate.

Grafana dashboards (as code, in `deploy/grafana/`):
- "SKYCOT overview" — RED metrics, signups, tenant count.
- "Provider spend" — per-tenant + workspace spend.
- "Action dispatch" — per-kind + success/failure breakdown.
- "Audit chain health" — verify-pass-rate over time.

Critical alerts (Alertmanager rules):
- Error rate > 1% over 5 min → PagerDuty.
- Provider latency p99 > 30s → Slack.
- Signup rate spike (10x baseline) → Slack (could be DoS or viral moment).
- Spend > $100/hr/tenant → Slack.

LOC: ~800.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/metrics_route.rs` — narrowly: ensure `/metrics` returns Prometheus-format. ≤80 modified lines.
- `crates/argo-telemetry/src/prometheus_exporter.rs` — extend with new metric kinds. ≤200 modified lines.
- `deploy/grafana/dashboards/skycot-overview.json` (NEW). ≤200 LOC.
- `deploy/grafana/dashboards/provider-spend.json` (NEW). ≤120 LOC.
- `deploy/grafana/dashboards/action-dispatch.json` (NEW). ≤120 LOC.
- `deploy/grafana/dashboards/audit-chain.json` (NEW). ≤100 LOC.
- `deploy/prometheus/scrape-config.yml` (NEW). ≤40 LOC.
- `deploy/alertmanager/rules.yml` (NEW). ≤80 LOC.
- `deploy/grafana/README.md` (NEW) — operator setup. ≤80 LOC.

## Acceptance criteria

- [ ] `/metrics` returns valid Prometheus exposition format.
- [ ] All listed metric series are populated.
- [ ] 4 Grafana dashboards import cleanly.
- [ ] Alertmanager rules validate.
- [ ] `cargo test -p argo-cli -p argo-telemetry metrics` passes.
- [ ] `cargo clippy -p argo-cli -p argo-telemetry --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-observability-prometheus-grafana`
- PR title: `obs: Prometheus + Grafana + alerts (codex-observability-prometheus-grafana)`

## Notes

- Use existing `argo-telemetry` crate; don't author parallel telemetry.
- Operator deploys Prometheus + Grafana on Fly.io alongside the gateway (or use Grafana Cloud free tier).
- PagerDuty integration is optional v1; Slack-only alerts acceptable.
