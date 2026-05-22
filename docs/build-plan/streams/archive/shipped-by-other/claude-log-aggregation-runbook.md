# claude-log-aggregation-runbook — RUNBOOK section on log aggregation patterns

**Owner**: claude. **Depends on**: existing RUNBOOK (`docs/skycot/RUNBOOK.md`)
— narrow extension only.

## Scope

Operators currently get raw stderr/stdout from `argo gateway serve`. There's
no guidance for shipping logs to a real aggregator (Loki, Grafana, ELK).
This brief adds a focused RUNBOOK section covering:

1. Structured JSON output mode (env var to enable)
2. File rotation patterns (logrotate snippet, journald hand-off)
3. Loki/Grafana shipping via Promtail (ready-to-paste config)
4. Field reference (which JSON keys are stable, which are best-effort)

This is **docs only**. No new code. The structured-JSON flag already exists
in `argo-cli` (verify with `rg -n 'tracing.*json' crates/argo-cli/src`); the
brief documents what's there, it does not add new code paths.

## Approach (concrete)

1. **Modify** `docs/skycot/RUNBOOK.md` — append a single new section
   `## Log aggregation` near the existing observability content. Approx
   80–120 lines. Sub-headings:
   - `### Structured JSON output` — env var, expected schema
   - `### File rotation` — logrotate config, journald option
   - `### Shipping to Loki + Grafana` — Promtail snippet, label conventions
   - `### Field reference` — table of stable keys (level, target, span, msg)
2. **NEW** `docs/skycot/log-aggregation/promtail-example.yaml` — ready-to-
   paste Promtail config (≤ 60 lines).

## Files you may touch

- `docs/skycot/RUNBOOK.md` — narrow append; do not refactor existing sections
- `docs/skycot/log-aggregation/promtail-example.yaml` (NEW)

## Files you must NOT touch

- Any code under `crates/` — this brief is docs-only
- Existing RUNBOOK sections — append-only
- The tracing setup in `argo-cli` — verify behavior, don't change it

## Acceptance criteria

- [ ] New `## Log aggregation` section in RUNBOOK reads end-to-end without
      pointing at code that does not exist
- [ ] All env vars referenced are confirmed live in current `argo-cli`
      (paste the `rg` command + result in the PR description)
- [ ] Promtail example YAML lints clean (`promtool check config` if available;
      otherwise YAML parse via `python3 -c 'import yaml; yaml.safe_load(open(...))'`)
- [ ] Field reference table covers at least: timestamp, level, target,
      span_name, span_id, trace_id, msg, fields
- [ ] No changes to code; CI green via docs-only path

## Branch + PR

- Branch: `claude-log-aggregation-runbook`
- PR title: `docs: log aggregation runbook section (claude-log-aggregation-runbook)`

## Notes for the implementer

- Cross-link to `mission-otlp-exporter` (#938 MERGED): traces ship via OTLP,
  logs ship via stdout/file → Promtail → Loki. Make this split explicit so
  operators don't conflate the two paths.
- Promtail label conventions: `app="argo"`, `component="gateway"`,
  `env="$ENVIRONMENT"`. Keep cardinality low — no per-session labels.
- If structured JSON is not yet wired (search-first), do **not** silently
  add code; surface the gap in the PR and stop. Brief is docs-only.
