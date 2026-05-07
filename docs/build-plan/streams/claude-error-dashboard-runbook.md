# Error triage runbook for common production patterns

**Owner**: claude. **Depends on**: none.

> SOURCE: production hardening Phase C. The codebase has
> structured error types (provider failures, schema-version
> mismatch, store lock contention, etc.) but no operator-facing
> guide on how to triage them when they show up in production.
> This brief writes that guide as a narrow extension to
> `docs/skycot/RUNBOOK.md`.

## Scope

Append an "Error triage" section to `docs/skycot/RUNBOOK.md`
that covers the three highest-frequency error patterns operators
encounter, with concrete diagnostic steps and remediation paths.

Patterns to document (one subsection each):

1. **Provider failures** — Anthropic / OpenAI 5xx, rate-limit
   429, timeout. Diagnostic: query `argo_errors_total{class="provider"}`
   over the last hour, correlate with provider status pages,
   check circuit breaker state (once
   `claude-circuit-breaker-providers` lands). Remediation:
   wait it out, fail traffic to the other provider, escalate
   to provider support.

2. **Schema-version mismatch** — store reports a migration
   version newer than the binary. Diagnostic: `argo store schema-info`,
   compare to binary's compiled-in expected version. Remediation:
   roll back the binary OR run `argo store migrate --to <ver>`
   if that direction is supported (check migration ledger).

3. **Lock contention** — store transactions blocking on each
   other; symptom is high p99 latency on `session.create` or
   `message.add`. Diagnostic: enable verbose store tracing,
   look for long-held transactions in the log; check for
   long-running `forge run` operations holding write locks.
   Remediation: identify the offending operation, kill it via
   `argo forge runs abort`, raise BUSY_TIMEOUT temporarily.

Each subsection: ~25-30 lines. No new code — pure docs.

## Files you may touch

- `docs/skycot/RUNBOOK.md` — narrowly: append a new
  "Error triage" section (~100 lines) after the existing
  Telemetry section. Use the same formatting style as the
  existing sections (H2 headers, fenced code blocks for shell
  commands, "Symptom / Diagnostic / Remediation" sub-headers).

## Files you must NOT touch

- ANY source file. This brief is documentation only.
- Any other doc file. RUNBOOK is the single operator entry
  point; don't fragment.

## Acceptance criteria

- [ ] New "Error triage" H2 section in RUNBOOK
- [ ] Three subsections covering the listed patterns
- [ ] Each subsection has Symptom / Diagnostic / Remediation
      sub-structure
- [ ] All shell commands actually exist on `main` today (verify
      via `argo --help`)
- [ ] Cross-links to relevant briefs where future work lands
      (e.g., circuit breaker, Prometheus dashboard)
- [ ] No emojis (project convention)
- [ ] Every modified file < 500 LOC (RUNBOOK target after
      this PR: well under)

## Branch + PR

- Branch: `claude-error-dashboard-runbook`
- PR title: `docs: error triage runbook for provider/schema/lock patterns (claude-error-dashboard-runbook)`

## Notes for the implementer

- Verify every CLI command you reference is real on current
  `main` — don't reference the not-yet-merged
  `argo costs anomalies` (separate brief). If you reference a
  command that depends on another in-flight brief, add a TODO
  pointer to that brief's branch name.
- Keep the tone operator-direct: "When you see X, run Y, then Z."
  Avoid theory.
- Do NOT add a "Common issues" or FAQ subsection that overlaps
  with these three patterns; merge improvements into the existing
  three.
