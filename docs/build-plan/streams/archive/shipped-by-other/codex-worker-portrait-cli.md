# V1.4 — Worker portrait CLI

**Owner**: codex. **Depends on**: H-worker-list-cli (#837),
H-budget-cli (#820), claude-anomaly-detector merged.

## Scope

`argo worker portrait <id>` produces a single-page markdown
summary of a worker — useful for operator at-a-glance,
Slack-paste, or the future web dashboard. Pulls together
data from existing read paths:

- Worker identity (id, persona name, role, tenant)
- Resolved auto-approval policy + lineage (worker → roster →
  workspace)
- Budget: spend today, this month, daily cap, headroom
- Recent action stats: pass/fail counts last 24h, average
  latency, average retries
- Open handoffs the worker is participating in
- Recent skill candidates proposed by this worker
- Anomaly events last 24h (if claude-anomaly-detector
  landed)
- Circuit breaker state (Closed / Open / HalfOpen)

Output formats:
- default: human-readable markdown
- `--json`: machine-readable envelope

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/worker_portrait.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod
- `crates/argo-cli/src/cli_worker_cmds.rs` — narrowly:
  `Portrait` subcommand variant
- `crates/argo-cli/tests/worker_portrait_smoke.rs` (NEW)

## Acceptance criteria
- [ ] All 8 sections render with real data
- [ ] Sections gracefully degrade if data unavailable (e.g.
      no anomaly events yet → "No anomalies detected" not
      a crash)
- [ ] `--json` follows universal envelope
- [ ] Markdown output is paste-friendly (no terminal escape
      codes)
- [ ] 4 unit tests + 1 CLI smoke test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-worker-portrait-cli`
- PR title: `cli: argo worker portrait — markdown summary per worker (codex-worker-portrait-cli)`
