# Tenant data isolation alert

**Severity**: P0. Any credible signal of cross-tenant data visibility
triggers this playbook. Severity does not de-escalate until isolation
is confirmed intact.

## Detection

- Cross-tenant capability denial spike in Prometheus (denials are the
  normal-path defense; a spike means something is asking that
  shouldn't be), OR
- Support ticket mentioning "I can see another customer's data" or
  "this isn't my workspace", OR
- Audit-chain row with `tenant_id` mismatch between actor and
  resource — a session for `<tenant-acme>` reading/writing rows
  scoped to `<tenant-northwind>`.

## Triage (≤5 min)

**Stop further writes from the suspected vector first**, then
investigate. Do not let triage queries emit cross-tenant data into
logs.

```bash
argo tenant quota --tenant <suspected-tenant> --pause
argo audit query --tenant <suspected-tenant> --since-minutes 60 --json \
  | jq '.[] | select(.actor_tenant_id != .resource_tenant_id) | {at, kind, session_id, actor_tenant_id, resource_tenant_id}'
argo incident bundle --since 24h --output /tmp/isolation-$(date -u +%Y%m%dT%H%M%SZ).tar.gz
```

Run the isolation regression suite against current main:

```bash
cargo test -p argo-orchestrator -- tenant_isolation 2>&1 | tail -40
```

## Mitigation

1. Emergency-pause affected tenants pending root cause (pause blocks
   new writes without destroying state).
2. If denials point to a specific connector or skill, disable it:
   `argo connector disable <name>`.
3. Notify affected tenants directly per
   [`customer-comms-templates.md`](../customer-comms-templates.md)
   — do not wait for full root cause.
4. File `docs/skycot/incidents/<YYYY-MM-DD>-tenant-isolation-<slug>.md`
   and engage the escalation chain.

## Rollback

Not applicable. Emergency-pause halts further writes; path forward is
root-cause + fix + verify. Restoring from a pre-incident backup is
only with security and legal sign-off, after the broken state is
preserved out of band.

## Notes

- Capability denials are first-line defense; their absence does not
  prove isolation, but a spike is a strong signal.
- Cross-tenant guarantee is core invariant #2 (CLAUDE.md); the PIR
  should track any architectural follow-ups.
