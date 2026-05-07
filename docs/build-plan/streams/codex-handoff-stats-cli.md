# V1.4 — Handoff stats CLI

**Owner**: codex. **Depends on**: H-orchestrator-persist (#806),
claude-handoff-routing-rules (#883) merged.

## Scope

Today operators see individual handoffs via `argo handoff list/show`
(#824). Missing: aggregate metrics — success rate, escalation rate,
median latency from request to resolution, top-handed-off-from / top-
handed-off-to workers.

Add:
- `argo handoff stats [--tenant <id>] [--since <duration>]
  [--by worker|skill|destination] [--json]` — markdown table of
  aggregate stats over the time window. Default `--since 7d`.
- `argo handoff stats --histogram [--bucket-mins N]` — ASCII
  histogram of handoff volume per bucket.

Sections in the markdown report:
- **Overview**: total handoffs, success rate, escalation rate,
  median latency
- **By worker (handed-off-FROM)**: top 5 workers initiating handoffs
- **By worker (handed-off-TO)**: top 5 workers receiving handoffs
- **By outcome**: resolved / escalated / abandoned / pending counts

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/handoff_stats.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod
- `crates/argo-cli/src/cli_handoff_cmds.rs` — narrowly: add `Stats`
  variant
- `crates/argo-store/src/handoff_stats_query.rs` (NEW) — pure
  aggregate queries (success_rate / median_latency / top_n_from /
  top_n_to / volume_per_bucket)
- `crates/argo-store/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/tests/handoff_stats_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo handoff stats` with default 7-day window returns 4-section
      markdown report
- [ ] `--by worker` re-groups stats by worker; `--by skill` by skill;
      `--by destination` by handoff target
- [ ] `--tenant t-1` filters to one tenant
- [ ] `--histogram --bucket-mins 60` returns 24-bucket ASCII histogram
      for past day
- [ ] `--json` envelope (structured stats, not the markdown rendering)
- [ ] 6 unit tests (each query helper independently) + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-handoff-stats-cli`
- PR title: `handoff: argo handoff stats — aggregate metrics + histogram (codex-handoff-stats-cli)`
