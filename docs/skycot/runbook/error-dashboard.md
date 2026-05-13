# SKYCOT Operator Runbook - Error triage

Triage guide for the three highest-frequency error patterns the
gateway and store surface in production: provider failures,
schema-version mismatch, and lock contention. Each subsection is
written in `Symptom -> Diagnostic -> Remediation` order so you can
read top-down once an alert fires. For the full table of contents,
see [RUNBOOK.md](../RUNBOOK.md).

For broader audit / metric tooling, see
[Audit and observability](observability.md). For OOM-style
worker restarts (a distinct path), see
[OOM-kill detection + worker restart](oom-restart.md). For startup
exit-78 failures, see
[Observability 9.7](observability.md#97-startup-health-check-exit-78).

---

## Provider failures

LLM provider 5xx, rate-limit 429, and timeout responses surface in
the routing layer. The runtime retries within a single turn; what
you see in audit is the *outcome* (fallback, success on retry, or
final failure), not every transient HTTP error.

**Symptom.** Worker turns stall, take far longer than usual, or
fail with `provider error: ...` in the assistant trace. Operator
dashboards see elevated `argo_evaluation_latency_seconds` and an
uptick in routing fallbacks.

**Diagnostic.**

```bash
# Routing decisions in the last hour - fallback events are the
# load-bearing signal here.
argo audit show --kinds routing.decision --since-minutes 60 --json \
  | jq '.tail[] | select(.payload_json | fromjson | .fell_back == true)'

# Cross-check against provider status pages:
#   Anthropic: https://status.anthropic.com
#   OpenAI:    https://status.openai.com

# Pull the gateway's text-format metrics for a recent counter delta
# (only when the gateway is up):
curl -s http://localhost:7676/metrics | grep -E '^argo_(actions|forge|worker)_'
```

**Remediation.** Three operator paths, in order of escalation:

1. **Wait it out.** Transient 5xx and 429s usually clear within
   minutes. The router retries inside a turn; you typically don't
   need to act.
2. **Force traffic to the other provider.** Set
   `ANTHROPIC_API_KEY=` (empty) or `OPENAI_API_KEY=` (empty) for the
   degraded side and restart the gateway. The startup check
   ([Observability 9.7](observability.md#97-startup-health-check-exit-78))
   refuses to boot when *neither* is set.
3. **Escalate.** Open a ticket with the affected provider, paste
   the failing `routing.decision` payloads, and pause non-critical
   verticals until status pages clear.

A persistent fallback pattern at the same provider is the signal
to inspect `argo provider breakers --json`. The provider
circuit-breaker work shipped under
`mission-circuit-breaker-wire-providers`; routing every live caller
through that wrapper is tracked by the follow-up
`mission-circuit-breaker-route-callers`.

---

## Schema-version mismatch

The store carries a forward-only `schema_version`. The binary
expects a compiled-in version; opening a store at a *newer*
version fails closed, and a *behind* version migrates up on boot
unless the operator has opted out.

**Symptom.** Gateway exits **78** with `STARTUP CHECK FAILED:
schema is ahead of the binary` or `STARTUP CHECK FAILED: schema is
behind the binary`. CLI commands that open the store (`argo audit
show`, `argo worker list`, ...) fail with a similar message.

**Diagnostic.**

```bash
# Schema number the *current* binary will accept on open:
argo db version

# Full status block (schema_version + per-table row counts):
argo db status --json | jq '{schema_version, db_path, rows}'

# Migration history is forward-only; see the operator reference
# for the fix-forward model and pre-upgrade backup recipe.
#   docs/skycot/operator/migrations.md
```

Compare the number `argo db version` prints against the
`schema_version` field in `argo db status --json`. If `status` is
greater, the DB is ahead of the binary; if less, it is behind.

**Remediation.**

- **DB is behind the binary.** Run `argo db migrate-up`. The
  command is idempotent; re-running on an already-migrated DB is a
  no-op.
- **DB is ahead of the binary.** Roll the binary forward to the
  release that authored that schema version (preferred), or roll
  the DB back with `argo db migrate-down --to <ver>` *only* if the
  in-flight migrations are reversible - most are not (additive-only
  migrations are forward-only by construction). When in doubt,
  restore the pre-upgrade backup and re-run the upgrade.
- **Unsure which version is correct.** Stop the gateway, snapshot
  the SQLite file (`cp $ARGO_HOME/sessions.db sessions.db.bak`),
  then upgrade the binary and run `argo db migrate-up`. Backups are
  the only supported rollback.

---

## Lock contention

SQLite is single-writer. When a long-running write (a forge run, a
bulk audit replay, a stuck connector) holds the writer, every
incoming `session.create` / `message.add` / approval write queues
behind it. The `busy_timeout` pragma (default 5000 ms) bounds the
wait before SQLite returns `SQLITE_BUSY`.

**Symptom.** p99 latency on `session.create` or `message.add`
climbs sharply; the gateway logs `database is locked` errors at
WARN; worker turns stall at the "write turn-end audit row" step.
`argo audit show --kinds lock.violate --since-minutes 30` returns rows.

**Diagnostic.**

```bash
# Running forge runs - these are the most common writer-holders.
argo forge runs list --status running --json | jq '.[] | {id, started_at, skill}'

# Recent lock-conflict audit rows:
argo audit show --kinds lock.violate --since-minutes 30

# WAL/checkpoint health (large WAL = a writer is not releasing):
argo doctor wal-stats --json | jq '{wal_pages, busy_count, checkpoint_micros}'

# Enable verbose store tracing for the next reproducer (restart
# the gateway with this set, then reproduce, then unset):
RUST_LOG=argo_store=debug,argo_runtime=info argo gateway start
```

**Remediation.**

1. **Abort the offending forge run.** Identify the stuck `run_id`
   from the diagnostic above, then
   `argo forge runs abort <run_id>`. The audit chain records the
   abort; the writer slot frees on the next checkpoint.
2. **Sweep stalled runs as a class.** If multiple runs are stuck
   (host reboot, oom, etc.), `argo forge runs sweep
   --older-than-minutes 30` flips every running row older than the
   threshold to `failed`. Pair with `argo forge runs abort-all`
   only as a last resort - it touches every tenant.
3. **Raise `busy_timeout` temporarily.** Set
   `[store.wal_tuning] profile = "aggressive"` in `argo.toml` *or*
   patch the active deployment's `busy_timeout` pragma upward
   (e.g. 10 000 ms) and restart. This buys you queue headroom while
   you find the root-cause writer; it does not fix the underlying
   contention. Revert to the `default` profile once the writer is
   gone - `aggressive` also flips `synchronous = OFF` and is
   dev-only.

Sustained lock contention at steady-state means a writer pattern
is wrong, not that the pragma is mistuned. Look for: a connector
holding a transaction across an HTTP round-trip, a forge run that
forgot to commit, or a long-running `argo audit replay`.

---
