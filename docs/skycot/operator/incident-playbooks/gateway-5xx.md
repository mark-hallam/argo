# Gateway 5xx spike

**Severity**: P1 (revenue + reputational impact). Escalate to P0 if
all routes are returning 5xx and `/livez` is also failing.

## Detection

- Prometheus alert `argo_request_errors_total > 1%` over a 5-minute
  window, OR
- `curl -fsS http://<gateway-host>/livez` returns non-200, OR
- status.skycot.com synthetic check reports red, OR
- Multiple support tickets within 10 min mentioning "everything is
  broken" or "site is down".

## Triage (≤5 min)

```bash
argo doctor --json | jq '.summary'
argo gateway routes-print --json | jq '.[] | select(.status != "ok")'
curl -fsS http://<gateway-host>/livez
curl -fsS http://<gateway-host>/readyz
tail -100 ~/.argo/gateway.log
argo audit verify-chain
```

`/livez` 200 + `/readyz` 503 = binary up but a dependency (DB,
provider, MCP) is failing. Identify the failing route from
`gateway routes-print`; that's the entry point for mitigation.

## Mitigation

- **One-route failure**: hotfix the offending handler, deploy, verify.
  Most callers continue degraded; no full rollback needed.
- **Whole-gateway failure**: graceful restart first
  (`argo gateway stop --grace 30s && argo gateway start`). If the
  binary won't restart cleanly, roll back to the prior image.

## Rollback

```bash
flyctl releases list
flyctl deploy --image <prior-image-sha>
```

After deploy, re-run `argo doctor --json` and confirm
`argo_request_errors_total` returns to baseline (<0.1%) within 5 min.

## Notes

- First 5xx in the log usually has the root-cause trace; subsequent
  ones are downstream retries. Read the earliest entry.
- If 5xx persists after restart **and** rollback, the cause is
  downstream — re-triage the downstream subsystem.
- Cross-link: runtime-level recipe in
  [`runbook/incident-response.md §5f`](../../runbook/incident-response.md).
- Cross-link: status page update flow in
  [`customer-comms-templates.md`](../customer-comms-templates.md).
