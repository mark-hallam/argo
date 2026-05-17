# Audit Chain Performance Benches

Run these before launch hardening changes that affect `audit_record`
growth, chain verification, or the operator audit feed:

```bash
cargo bench -p argo-audit --bench chain_perf
```

Criterion writes HTML reports under
`target/criterion/{verify_session,verify_all,audit_feed_since_24h_raw_fetch}/`.
Use the command output for the PR summary table and keep the HTML when
you need regression details.

The bench profiles cover:

- `verify_session`: one session with 1k, 10k, 100k, and 1M chained rows.
- `verify_all`: 10, 100, and 1000 sessions with 1k rows each.
- `audit_feed_since_24h_raw_fetch`: the gateway feed's raw SQL window
  shape at 1k, 10k, and 100k rows.

For a fast compile/runtime smoke, run:

```bash
ARGO_AUDIT_CHAIN_PERF_SMOKE=1 cargo bench -p argo-audit --bench chain_perf -- --test
```

Interpret the curve, not one sample. Sublinear or near-linear growth is
acceptable for launch. Any 100k-row profile over 5s should trigger a
follow-up index brief; include the slow query, scale point, and Criterion
mean in that brief. This bench intentionally does not change schema.
