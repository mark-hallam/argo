# V2.0 — Dreams memory summarize (Integration A)

**Owner**: claude. **Depends on**: F-memory-layering (#786),
H-orchestrator-persist (#806) merged. From dreams investigation
`docs/skycot/DREAMS_AND_RUNTIME_SWARM.md` Integration A.

## Scope

Today the memory layer's summary tier (`argo-memory::summary`)
generates summaries via existing LLM call patterns. The Claude
Dreams API (https://platform.claude.com/docs/en/managed-agents/dreams)
is purpose-built for this — background reflection over a worker's
memory + ≤100 sessions producing curated outputs. Better quality,
similar cost (~$0.75/dream), non-destructive (original memory
unchanged).

Replace the existing `summary.rs` LLM-call path with a Dreams-driven
backend. Old path stays available behind a feature flag for
fallback.

- New `argo-memory::dreams_backend` module wrapping the Dreams API
- Trigger: nightly per-worker tick + on-demand
  `argo memory dream-now <worker>`
- Output: a Dreams-summary memory row tagged
  `kind: dream_summary, source: dreams_api`
- Cost guardrail: per-worker daily cap ($5 default, env override
  `ARGO_DREAMS_DAILY_CAP`)
- v1 reliability prerequisite: do NOT enable for workers with
  open anomaly events (anomaly-detector #847 once that lands)
  — a misbehaving worker would poison its own dream

## Files you may touch
- `crates/argo-memory/src/dreams_backend.rs` (NEW)
- `crates/argo-memory/src/summary.rs` — narrowly: feature-flag
  the Dreams path; old LLM path stays as fallback
- `crates/argo-memory/src/lib.rs` — narrowly: re-export +
  feature flag
- `crates/argo-memory/Cargo.toml` — narrowly: feature
  `dreams-backend` (default off until anomaly-detector lands)
- `crates/argo-cli/src/handlers/memory_dream_now.rs` (NEW)
- `crates/argo-cli/src/cli_memory_cmds.rs` — narrowly:
  `DreamNow` subcommand variant
- `crates/argo-cli/tests/memory_dreams_smoke.rs` (NEW —
  gated on `ANTHROPIC_API_KEY`)

## Acceptance criteria
- [ ] Dreams API call works against current Anthropic endpoint
- [ ] Summary persists with kind=dream_summary
- [ ] Per-worker daily cap honored (refuses past cap)
- [ ] Anomaly-event guard: no Dreams for workers with open
      anomalies (refuses with clear error)
- [ ] Feature flag gates the path (default: off)
- [ ] 6 unit tests (mocked Dreams API) + 1 integration smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-dreams-memory-summarize`
- PR title: `memory: Dreams API backend for summary tier (claude-dreams-memory-summarize)`
