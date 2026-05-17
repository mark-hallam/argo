# Stripe webhook failures

**Severity**: P1 (billing state drifts from Stripe = revenue impact).

## Detection

- `argo billing dlq list` reports unresolved events, OR
- Stripe dashboard → Developers → Webhooks shows non-2xx delivery
  failures, OR
- Customer reports billing discrepancy (product state ≠ Stripe state).

## Triage (≤5 min)

```bash
argo billing dlq list --json | jq '.[] | {event_id, kind, failed_at, retry_count, last_error}'
argo billing dlq list --since 24h --include-resolved
tail -200 ~/.argo/gateway.log | grep -i stripe
```

For a specific failing event:

```bash
argo billing dlq list --json | jq '.[] | select(.event_id == "evt_<id>")'
```

Confirm: webhook secret current; tenant row exists with expected
state; event not already applied (replay-safe).

## Mitigation

After fixing the root cause, replay dead-lettered events:

```bash
# Single event after targeted fix
argo billing replay --event-id evt_<id>

# Bulk replay within a time window
argo billing replay --since 24h --limit 100
```

If a DLQ entry was reconciled out of band, mark resolved without
replay:

```bash
argo billing dlq resolve --event-id evt_<id> --note "reconciled manually 2026-05-17"
```

## Rollback

Not usually rollbackable — Stripe is upstream source of truth. The
"rollback" is to reconcile local state to match Stripe, never the
other way. If a bad handler deploy applied wrong state, revert the
handler and replay from the DLQ.

## Notes

- DLQ persists events; nothing is lost on a webhook outage so long as
  Stripe retried into the dead-letter sink.
- Stripe retries with exponential backoff for ~3 days, giving a
  generous mitigation window.
- Cross-link: [`docs/skycot/operator/billing-recovery.md`](../billing-recovery.md)
  for the full reconcile procedure.
