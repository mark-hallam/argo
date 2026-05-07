# claude-spend-forecast-cli — `argo costs forecast --horizon eom`

**Owner**: claude. **Depends on**: existing spend rows in argo-store
(verify table name with `rg -n 'spend' crates/argo-store/src`).

## Scope

Operators run `argo costs` to see actuals. This brief adds a forecast view:
project end-of-day, end-of-month, end-of-quarter spend from current
trajectory using linear extrapolation over a configurable lookback.

Linear extrapolation only — no ML. The math is rate × time-remaining,
where rate = sum(spend in lookback) / lookback_seconds.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/costs_forecast.rs` (~280 LOC):
   - `pub fn run(horizon: Horizon, lookback: Duration, format: OutputFormat) -> ArgoResult<()>`
   - `Horizon` enum: `Eod`, `Eom`, `Eoq`
   - `compute_forecast(spend: &[SpendRow], now: DateTime<Utc>, horizon: Horizon, lookback: Duration) -> Forecast`
   - Output: actual-so-far, projected-additional, projected-total,
     confidence-band (±20% as a flat heuristic for v1, documented as such)
2. **Modify** `crates/argo-cli/src/cli.rs`:
   - Add `Forecast { horizon, lookback, format }` variant under `Costs`
3. **Modify** `crates/argo-cli/src/main.rs`:
   - Dispatch arm calling `costs_forecast::run`

## Files you may touch

- `crates/argo-cli/src/handlers/costs_forecast.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/costs_forecast_tests.rs` (NEW, ≤ 250 LOC)
  - 7 unit tests:
    - eod horizon with 1h lookback projects correctly
    - eom horizon spans month boundary
    - eoq horizon spans quarter boundary
    - empty spend rows → forecast of 0 (not panic)
    - lookback longer than available data clamps to available
    - leap year EOM is correct (Feb 29)
    - JSON format round-trips
- `crates/argo-cli/src/cli.rs` — narrow: 1 subcommand
- `crates/argo-cli/src/main.rs` — narrow: 1 dispatch arm

## Files you must NOT touch

- argo-store schema — read existing spend rows only
- argo-types — keep `Forecast` struct private to handler
- Other costs handlers — leave them alone

## Acceptance criteria

- [ ] `argo costs forecast --horizon eom` prints a markdown table:
      actual-so-far, projected-additional, projected-total, ±band
- [ ] `--horizon eod|eom|eoq` all supported
- [ ] `--lookback 24h|7d|30d` overrides default (24h)
- [ ] `--format json` emits structured output
- [ ] Empty data does not panic; renders zeros + a "no data" note
- [ ] Math is documented in a doc-comment on `compute_forecast`
- [ ] 7 unit tests
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-spend-forecast-cli`
- PR title: `costs: argo costs forecast horizon-based projection (claude-spend-forecast-cli)`

## Notes for the implementer

- Use `chrono` for date math (already a workspace dep — verify before adding).
- The ±20% band is a deliberate flat heuristic for v1; document as such in
  the doc-comment so a future PR can replace it with stddev over windows.
- Eoq math: quarter = `((month - 1) / 3) * 3 + 3`, last day = end of that
  month. Cover the boundary in tests.
- Output column widths: keep table readable in 80-char terminals.
