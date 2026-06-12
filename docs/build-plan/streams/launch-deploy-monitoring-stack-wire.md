# launch-deploy-monitoring-stack-wire

**Owner**: claude or codex — ops observability; conductor review before merge.
**Depends on**: none.
**Authored against**: origin/main `7274fb673` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh launch-deploy-monitoring-stack-wire

## Problem (launch audit 2026-06-12)

Alert rules (`deploy/alertmanager/rules.yml`: 5xx rate, p99, signup spike,
tenant spend burn), scrape config and 4 Grafana dashboards are AUTHORED but
nothing runs them: no prometheus/alertmanager in any compose file, no
receiver configured — an outage or spend-burn is invisible until a
customer emails. Dashboards never validated against live metric labels.

## Fix

Add prometheus + alertmanager (+ optional grafana) services to
`deploy/docker-compose.yml` (profile-gated, e.g. `--profile monitoring`),
an `alertmanager.yml` with an SMTP/webhook receiver fed by env (reuses the
SES creds), scrape the gateway /metrics; validate the 4 dashboards' queries
against a live scrape (fix label drift); document the 3 operator steps in
DEPLOYMENT.md + PRODUCTION-RUNBOOK. Compose-config validation test
(`docker compose config -q`) in the smoke script.

## Acceptance
1. `docker compose --profile monitoring config -q` green in smoke.
2. Rules load (amtool check-config or promtool test green in CI-less
   script form); dashboard queries match live metric names (documented
   check). 3. Docs updated same commit.

~100-180 LOC config. **🟠 P2 — turns authored monitoring into running
monitoring.**
