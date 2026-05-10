# Mission load-bearing wire — Forge cost attribution per run (skill / vertical / tenant)

**Owner**: claude. **Depends on**: `argo-forge::run::ForgeRun` SHIPPED (verified at `crates/argo-forge/src/run.rs:105`); `budget_usd_micros` field exists (line 71); LLM provider cost-per-token tracking SHIPPED.
**Authored against**: origin/main `cbcf8498` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-forge-cost-attribution`.

> SOURCE: MISSION_BACKLOG.md Section 6 #206 ("Build cost-attribution per Forge run"). Today `ForgeRun.budget_usd_micros` caps spending but the **actual spend per run isn't tracked** beyond aggregate counters. Operators tuning Forge budget can't see per-skill or per-vertical cost breakdowns — only total burn. Mission Claim 4 ("skills shared and learned") needs cost attribution so operators can identify which skills cost too much to optimize and which return value.

## Scope

Extend `ForgeRun` and Forge-result persistence with structured cost attribution:

```rust
pub struct ForgeCost {
    pub llm_input_tokens: u64,
    pub llm_output_tokens: u64,
    pub llm_cache_read_tokens: u64,    // separate (cheaper) from input
    pub llm_cost_usd_micros: u64,       // computed from per-model pricing
    pub e2b_seconds: u64,               // sandbox wall-clock if used
    pub e2b_cost_usd_micros: u64,
    pub total_cost_usd_micros: u64,     // sum of above
}

pub struct ForgeRun {
    // ... existing fields ...
    pub cost: ForgeCost,                // NEW
}
```

Each provider call updates the in-flight `ForgeCost` accumulator; on `finalize_run` the totals are stamped into the persisted `forge_run` row.

NEW columns on `forge_run` table (migration vN+1):
- `llm_input_tokens INTEGER NOT NULL DEFAULT 0`
- `llm_output_tokens INTEGER NOT NULL DEFAULT 0`
- `llm_cache_read_tokens INTEGER NOT NULL DEFAULT 0`
- `llm_cost_usd_micros INTEGER NOT NULL DEFAULT 0`
- `e2b_seconds INTEGER NOT NULL DEFAULT 0`
- `e2b_cost_usd_micros INTEGER NOT NULL DEFAULT 0`
- `total_cost_usd_micros INTEGER NOT NULL DEFAULT 0`

CLI: `argo forge cost-report [--skill <id>] [--vertical <id>] [--tenant <id>] [--since <ts>]` prints per-skill aggregate spend over time window (count of runs, mean cost, p99, total). JSON envelope on `--json`.

This is NOT in scope: per-token cost prediction (only post-hoc accounting); cost alerting (separate brief if needed); Stripe/billing integration; cross-tenant cost aggregation (per-tenant scoped only).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__forge_run_cost.sql` (NEW) — ALTER TABLE adds 7 columns idempotently via `column_exists` PRAGMA probe. **Implementer MUST verify next free slot** via `grep -E "v4[6-9]|v5[0-9]" docs/build-plan/BUILD_STATUS.md` to avoid collision with in-flight briefs (mission-escalation-on-no-reply may use v50/51).
- `crates/argo-store/src/migrations/v<next>.rs` (NEW) — versioned migration.
- `crates/argo-forge/src/run.rs` — narrowly scoped: ADD `ForgeCost` struct, ADD `cost: ForgeCost` field to `ForgeRun`, ADD `accumulate_llm_call(input, output, cache_read, model)` helper on ForgeRun that updates the accumulator. File currently ~200 LOC — verify under 500.
- `crates/argo-forge/src/cost.rs` (NEW, ≤ 250 LOC) — per-provider cost computation (Anthropic pricing table, E2B pricing). Pricing is hardcoded constants for v1; future brief covers pulling from `argo-provider` manifests.
- `crates/argo-forge/src/run_tests.rs` (or sibling) — 6 unit tests: accumulator-sums-correctly; cache-read-cheaper-than-input; e2b-time-rounded-up; total-equals-sum; zero-tokens-zero-cost; multi-call-accumulates.
- `crates/argo-cli/src/handlers/forge_cost_report.rs` (NEW, ≤ 200 LOC) — `argo forge cost-report` CLI handler.
- `crates/argo-cli/src/cli_forge_cmds.rs` (or wherever forge subcommands live — verify in first 5min) — narrowly scoped: ADD `cost-report` enum variant.
- `crates/argo-cli/tests/forge_cost_smoke.rs` (NEW, ≤ 120 LOC) — integration smoke: persist 3 forge runs with known costs, run cost-report, assert aggregate matches.

## Files you must NOT touch

- `crates/argo-forge/src/forge.rs` — `finalize_run` core logic; only the cost-stamping line is added
- `crates/argo-skill/src/quality.rs` — quality is upstream; orthogonal to cost
- `crates/argo-provider/**` — provider machinery is upstream; pricing constants come from public Anthropic docs for v1
- Any non-forge/non-store/non-cli crate

## Acceptance criteria

- [ ] `ForgeCost` struct with all 7 fields exists
- [ ] `ForgeRun.cost` field populated on finalize
- [ ] Migration adds 7 columns idempotently
- [ ] Per-call accumulator updates input/output/cache_read tokens + cost
- [ ] Pricing table: Anthropic Sonnet/Opus current pricing (hardcoded; document the model+date stamp)
- [ ] E2B cost computed from seconds × $0.00029/s (current E2B rate; hardcoded)
- [ ] `argo forge cost-report` CLI works with `--skill`/`--vertical`/`--tenant`/`--since` filters
- [ ] JSON output for scripting
- [ ] 6 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-forge -p argo-cli` clean
- [ ] `cargo clippy -p argo-store -p argo-forge -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-store -p argo-forge -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-forge-cost-attribution`
- PR title: `forge: per-run cost attribution + argo forge cost-report CLI (mission-forge-cost-attribution)`

## Notes for the implementer

- Pricing constants are intentionally hardcoded for v1 — Anthropic and E2B rates rarely change, and pulling from configurable manifests adds complexity for marginal benefit. Document the source/date in the cost.rs module doc.
- The `cache_read_tokens` field is critical: cached reads are 10x cheaper than uncached inputs, so a Forge run that hits cache heavily looks dramatically cheaper. Operators tuning prompt structure need this number to be honest.
- The CLI aggregation defaults to mean + total; p99 latency-style isn't meaningful for cost (no real outliers to chart). Keep the output schema simple.
- Don't pull cost into the QualityScore signal here — that's a downstream brief (`mission-quality-cost-weighted-promote` or similar).
