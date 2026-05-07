# claude-availability-sla-tracking — `argo doctor sla` rolling-month SLO

**Owner**: claude. **Depends on**: existing gateway request log
(used to derive uptime: any 5xx or transport failure is downtime).

## Scope

Operators want to know "are we hitting 99.9% this month?" without
spinning up Grafana. This brief computes a rolling-month uptime
percentage from the gateway request log and prints it via
`argo doctor sla`.

Uptime model (v1, deliberately simple): bucket time into 1-minute slots.
A slot is "down" if any request in that slot returned 5xx or transport
error AND no request in that slot returned 2xx. Otherwise "up". Empty
slots are excluded from the denominator (no traffic = no signal).

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/sla.rs` (~280 LOC):
   - `pub fn run(target_pct: f64, window: Duration, format: OutputFormat) -> ArgoResult<()>`
   - `compute_sla(store: &Store, now: DateTime<Utc>, window: Duration) -> SlaReport`
   - `SlaReport { up_minutes, down_minutes, observed_minutes, uptime_pct, target_pct, error_budget_remaining_minutes }`
   - Output: markdown summary + a small ASCII heatmap of last 24h by hour
2. **Modify** `crates/argo-cli/src/cli.rs`:
   - Add `Sla { target, window, format }` variant under `Doctor`
3. **Modify** `crates/argo-cli/src/main.rs`: 1 dispatch arm.

## Files you may touch

- `crates/argo-cli/src/handlers/sla.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/sla_tests.rs` (NEW, ≤ 280 LOC) — 8 tests:
  - empty log → "n/a" (not 100%)
  - all 2xx → 100%
  - 1 down minute in 1000 observed → 99.9%
  - mixed 5xx and 2xx in same minute → up (any-success rule)
  - error budget calculation against 99.9% target
  - rolling 30d window clipping
  - heatmap renders 24 cells
  - JSON round-trip
- `crates/argo-cli/src/cli.rs` — narrow: 1 subcommand
- `crates/argo-cli/src/main.rs` — narrow: 1 dispatch arm

## Files you must NOT touch

- argo-store schema — read existing request log only
- argo-types — keep `SlaReport` private to handler
- Other doctor subcommands — leave them alone

## Acceptance criteria

- [ ] `argo doctor sla` defaults to 30d window, 99.9% target
- [ ] Output: uptime%, observed minutes, error-budget-remaining minutes,
      24h ASCII heatmap
- [ ] `--target 99.95` and `--window 7d` work
- [ ] `--format json` emits structured fields
- [ ] Empty / no-traffic minutes excluded from denominator
- [ ] Exit code 1 if uptime% < target (so CI/cron can alert)
- [ ] 8 unit tests
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-availability-sla-tracking`
- PR title: `sla: argo doctor sla rolling-window uptime (claude-availability-sla-tracking)`

## Notes for the implementer

- The "any 2xx in slot → up" rule is intentional: a single user retry that
  succeeds means the system was reachable. Document this on the function.
- Error budget math: `(1 - target) * observed_minutes - down_minutes`.
  Negative = budget exhausted; surface as `EXCEEDED` with sign.
- Heatmap cells: 24 cells, one per hour of last 24h. Use unicode block
  shades `▁▂▃▄▅▆▇█` for severity (or ASCII `.:|#` if blocks are noisy in
  CI logs — prefer the block characters; they render in modern terminals).
- Keep the slot bucketing (1-minute) as a constant — easy to refactor
  later if we need finer granularity.
