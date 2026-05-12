# Bench baselines

Committed criterion-bench baselines used by `.github/workflows/bench.yml`
(currently `.disabled` while GitHub Actions billing is frozen). The
workflow runs `scripts/bench-compare.sh` against `main.json` to detect
regressions; a delta > 5% on any bench fails the workflow step.

## Files

- `main.json` — baseline targeting `main`. Refresh via
  `scripts/bench-baseline-refresh.sh` after a deliberate perf-affecting
  merge, then commit alongside that PR.

## Placeholder caveat (v1)

The shipped `main.json` carries **placeholder** numbers and a
placeholder `host_fingerprint`. It is not authoritative; operators
refresh it once the first real CI run has produced realistic numbers
on the workflow runner. Until refreshed, the CI step uses
`--allow-cross-host` so the placeholder fingerprint does not block.

## Schema

See `scripts/bench-record.sh` for the canonical shape:

```jsonc
{
  "git_sha":          "<short sha or 'placeholder'>",
  "recorded_at":      "<ISO 8601 UTC>",
  "host_fingerprint": "<uname-sm + hash of CPU model>",
  "results": { "<group>::<case>": { "ns": <n>, "stderr_ns": <n> } }
}
```

## See also

- `scripts/bench-record.sh` / `scripts/bench-compare.sh` /
  `scripts/bench-baseline-refresh.sh`
- `docs/skycot/runbook/perf-benches.md` → Regression detection.
