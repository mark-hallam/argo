# mission-forge-spend-polling-race-fix — Fix forge_run.spend_usd_micros polling race

**Owner**: claude. **Depends on**: nothing (bug fix).
**Authored against**: origin/main `7598d5d1` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-forge-spend-polling-race-fix`.

## Scope

MISSION_BACKLOG item 268. The `forge_run.spend_usd_micros` field is polled by a budget gate; a race exists where the polling read can see a stale value between a provider call's spend accumulation and its commit-to-DB. The race lets a budget-exhausted forge run a single extra step. Fix: atomic compare-and-swap on the spend field, or single-writer transaction wrap around accumulate+commit.

LOC: ~400.

## Files you may touch

- `crates/argo-forge/src/spend.rs` — TOUCH. Wrap accumulate+commit in single transaction OR convert to atomic CAS. Verify exact path at claim time. ≤40 modified lines.
- `crates/argo-forge/src/spend_tests.rs` — TOUCH. Add race-test (synthetic concurrent commits). ≤30 modified lines.

## Files you must NOT touch

- Budget gate logic (reader side); provider call accumulation.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Synthetic 100-iteration concurrent commit test produces correct final value (no lost updates).
- [ ] Budget gate refuses one-extra-step in the race scenario.
- [ ] Existing forge tests still green.
- [ ] No regression in forge happy-path perf (within 5%).
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-forge-spend-polling-race-fix`
- PR title: `Fix forge_run.spend_usd_micros polling race (mission-forge-spend-polling-race-fix)`

## Notes for the implementer

- Mission-claim alignment: production-hardening (operational correctness, supports all mission claims).
- Authored from MISSION_BACKLOG.md [NEW] item — no prior brief existed; check `rg 'pub (struct|enum|fn) <Name>'` for any new names introduced before declaring (drift prevention rule).
