# Audit chain break

**Severity**: P0. Evidence-quality issue. Treat as a forensic event,
not a normal operational defect.

## Detection

- `argo audit verify-chain --json | jq '.ok'` returns `false`, OR
- Prometheus alert `argo_audit_chain_breaks_total > 0`, OR
- `argo doctor --json` reports `audit_chain: broken`.

## Triage (≤5 min)

```bash
argo audit verify-chain --json | jq '{ok, head_seq, first_break_seq, kind}'
argo audit query --row <first_break_seq - 1> --row <first_break_seq + 1>
argo incident bundle --since 24h --output /tmp/audit-break-$(date -u +%Y%m%dT%H%M%SZ).tar.gz
```

Capture: which session, break kind (hash mismatch / sequence gap /
signature failure), wall-clock window (compare `at` on the two
divergent rows).

## Mitigation

**Do NOT delete the broken row. Do NOT attempt in-place repair.**
A chain that keeps accepting writes after a break corrupts further;
in-place edits destroy evidence.

1. Stop the gateway: `argo gateway stop --grace 0`.
2. Preserve the audit DB (snapshot the file or block-storage volume
   out of band, with timestamp).
3. Escalate per the [escalation chain](../incident-response.md#escalation).
4. Open `docs/skycot/incidents/<YYYY-MM-DD>-audit-chain-break-<slug>.md`.
5. Chain repair happens out of band, under security review, with the
   original DB preserved untouched. Repair lands as a new chain
   continuation, not as edits to the broken row.

## Rollback

Not applicable. Preserve the broken chain as-is. Restoring from a
pre-break backup is acceptable **after** the broken DB is snapshotted,
never instead of preserving it.

## Notes

- Chain breaks imply software defect at the write path, disk
  corruption, or tampering. All three require investigation.
- Cross-link: runtime triage in
  [`runbook/incident-response.md §8b`](../../runbook/incident-response.md).
