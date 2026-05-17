# Quota / rate-limit incidents

**Severity**: P2 (single-tenant) escalating to P1 if multi-tenant or
infra-wide. P0 if the cap enforces a safety boundary now defeated.

## Detection

- Prometheus alert `argo_metric_cardinality_capped_total` non-zero
  (observability throttle engaged), OR
- `tenant_rate_limit_hits_total` spike for a single tenant or
  connector, OR
- Customer reports "my agent suddenly stopped working" with a
  rate-limit-shaped error.

## Triage (≤5 min)

```bash
argo doctor --json | jq '.quotas'
argo tenant quota --tenant <tenant-acme> --json
argo audit query --tenant <tenant-acme> --since-minutes 30 --json \
  | jq '.[] | {at, kind, worker_id}'
```

Identify which tenant, which connector/action, and which pattern: a
retry loop repeats the same `kind` on sub-second cadence; a buggy
skill burns quota from one action; a legitimate burst varies `kind`.

## Mitigation

- **Legitimate burst**: temporary per-tenant cap raise:

  ```bash
  argo tenant quota --tenant <tenant-acme> --raise --duration 1h
  ```

- **Buggy skill**: `argo tenant quota --tenant <tenant-acme>
  --pause`, identify the skill, fix or pin to the prior revision,
  resume.
- **Retry loop**: contain the downstream failure (real root cause);
  rate-limit hits are symptom.
- **Cardinality cap**: identify the high-cardinality label, fix at
  the metric source, clear cap state at the next gateway restart.

## Rollback

Revert temporary quota overrides once root cause is fixed:

```bash
argo tenant quota --tenant <tenant-acme> --reset
```

Skill revisions revert through the marketplace pinning UI.

## Notes

- A rate-limit alert is usually a downstream signal — find the
  upstream cause before normalizing the cap.
- Cross-link: cardinality cap design in
  [`runbook/observability.md`](../../runbook/observability.md).
