# SKYCOT Operator Runbook

**Status**: v1 customer-launch — 28 PRs merged, 4-day swarm
delivered the full Worker OS stack, e2e gate (claude-e2e-integration-test #810) passes
offline.

A walkthrough from `cargo install` to your first auto-approved
action and a live audit trail. Written for an operator who has
never seen SKYCOT — by the end you'll know what to install,
how to drive it, and where to look when something goes wrong.

If you want the *why*, read [MISSION.md](MISSION.md). This
document is the *how*.

This runbook was split into topic-grouped sub-files so each one
stays under the project's 500-LOC-per-file ceiling. The sections
below are unchanged in content and ordering — they just live in
separate files now.

---

## Topics

1. [Getting started](runbook/getting-started.md) — what you're
   getting (§1), install (§2), bootstrap (§3).
2. [Workers and approvals](runbook/workers.md) — generate your
   first worker (§4), configure approvals (§5).
3. [Actions, skills, and handoffs](runbook/actions.md) — drive
   the first action (§6), skill learning loop (§7), multi-worker
   handoff (§8).
4. [Audit and observability](runbook/observability.md) — `argo
   audit query`, `argo doctor`, Prometheus, OTLP exporter, live
   audit stream, coverage, common failure modes (§9).
5. [Security](runbook/security.md) — gateway bearer-token auth,
   public read-only allowlist, integration-test bypass (§11).
6. [Production deploy](runbook/production-deploy.md) — Nginx,
   Caddy, systemd, and Docker Compose recipes for putting the
   gateway in front of real traffic (§12).
7. [Reference](runbook/reference.md) — where to next (§10), CLI
   command index (Appendix A), customer-demo file checklist
   (Appendix B).

---

## Quick links by task

- **First time installing?** → [Getting started](runbook/getting-started.md)
- **Generating a worker from a charter?** →
  [Workers and approvals §4](runbook/workers.md#4-generate-your-first-worker)
- **Setting up auto-approval?** →
  [Workers and approvals §5](runbook/workers.md#5-configure-approvals)
- **Driving an action through approve → execute → complete?** →
  [Actions §6](runbook/actions.md#6-drive-the-first-action)
- **Approving a learned skill proposal?** →
  [Actions §7](runbook/actions.md#7-skill-learning-loop)
- **Querying the audit ledger?** →
  [Observability §9.1](runbook/observability.md#91-argo-audit-query)
- **Wiring up Prometheus or OTLP?** →
  [Observability §9.3](runbook/observability.md#93-prometheus)
- **Shipping logs to Loki / Grafana / ELK?** →
  [Observability §9.8](runbook/observability.md#98-log-aggregation)
- **Gateway exits 78 on boot?** →
  [Observability §9.7](runbook/observability.md#97-startup-health-check-exit-78)
- **Setting a gateway bearer token?** →
  [Security §11.1](runbook/security.md#111-gateway-bearer-token-auth)
- **Putting the gateway behind a reverse proxy?** →
  [Production deploy §12](runbook/production-deploy.md#12-production-deploy)
- **Customer-demo readiness check?** →
  [Reference Appendix B](runbook/reference.md#appendix-b--file-checklist-for-a-customer-demo)

---

## Approvals JSON API

The gateway exposes a JSON wrapper over the worker-action
approval state machine at `/api/v1/approvals` (the legacy HTML
`/inbox` route is preserved separately). All endpoints require
`Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN` when the
gateway bearer-auth token is configured.

```bash
# List pending approvals (default 50 per page, max 100):
curl -sH "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  http://127.0.0.1:8734/api/v1/approvals

# Resume from prior page using next_cursor from the response body:
curl -sH "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  "http://127.0.0.1:8734/api/v1/approvals?limit=50&cursor=<token>"

# Approve / refuse a specific approval:
curl -sX POST -H "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  -d '{"decision":"approve"}' \
  http://127.0.0.1:8734/api/v1/approvals/<id>/decide

# Reverse a decision within the 30-second undo window:
curl -sX POST -H "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  http://127.0.0.1:8734/api/v1/approvals/<id>/undo
```

Error envelopes follow `{ "error": { "code": "...", "message":
"..." } }`. Standard codes: `bad_request` (400),
`approval_not_found` (404), `already_decided` (409),
`undo_window_expired` (410), `internal_error` (500).
