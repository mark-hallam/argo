# codex-notify-fallback-risk-filter — Apply matches_risk in notification fallback dispatch (F-05 P1)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `58e853b6` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-notify-fallback-risk-filter`.
**Source**: F-05 P1 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Mission alignment

- **Claim 6 (multi-channel approvals):** Each `NotifyChannel` handles risk-appropriate routing per operator preference (`docs/skycot/MISSION.md:115`, `:117`). The approval contract is "channel-appropriate gate," not "send somewhere" (`docs/skycot/MISSION.md:108`).

## Scope

P1 correctness bug. The normal fan-out path applies `matches_risk` filter (`crates/argo-cli/src/handlers/worker_actions_notify.rs:228`) but the fallback path does not:

1. `try_dispatch_on_ask` passes `config.channels` directly into fallback dispatch (`worker_actions_notify.rs:277`, `:283`, `:285`, `:288`).
2. `worker_actions_notify_fallback.rs:35` documents "risk filtering happens in the caller" — but the caller doesn't.
3. `run_one_attempt` picks the first channel kind match without checking `risk_level` (`worker_actions_notify_fallback.rs:74`, `:81`).

Result: low-risk action can be routed to high-risk-only fallback channel, or a high-risk action skips the intended high-risk channel because the first kind match wins.

Tests: normal-dispatch risk filtering is tested (`worker_actions_notify_tests.rs:236`); fallback fixture has no risk filters (`worker_actions_notify_fallback_tests.rs:89`) — so the bug went unobserved.

Scope:
- Filter `channel_refs` by `matches_risk` BEFORE fallback dispatch — either in `try_dispatch_on_ask` before calling `dispatch_with_fallback`, or move risk matching into `dispatch_with_fallback` itself. The latter is cleaner (single ownership of risk logic).
- Add regression test fixture in `worker_actions_notify_fallback_tests.rs` with two channels of the same kind but mismatched risk filters — assert fallback selects the risk-matching one.
- Add second test: low-risk action with only high-risk-tagged channels in fallback → must NOT send to any (or fail cleanly, per fallback policy).

LOC: ~150.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_notify_fallback.rs` — accept `risk_level` (or fold `matches_risk` here); apply filter in `run_one_attempt`. ≤80 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_notify.rs` — narrowly: if approach (1) chosen, filter channels before dispatch call at `:277-288`. ≤30 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_notify_fallback_tests.rs` — add regression tests. ≤80 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_notify_tests.rs` — narrowly: confirm normal-path tests still pass after refactor. ≤20 modified lines.

## Files you must NOT touch

- `crates/argo-channels/**` — channel implementations are correct.
- The `matches_risk` predicate definition itself — reuse it.
- `notify_*.rs` per-channel files (slack, email, sms, etc.) — out of scope.

## Acceptance criteria

- [ ] Fallback dispatch filters `channel_refs` by `matches_risk(channel.risk_level, action.risk_level)` BEFORE selecting a channel kind.
- [ ] Regression test: 2 channels same kind, different risk filter → correct one wins.
- [ ] Regression test: low-risk action + only high-risk-only fallback channels → no send (clean degradation).
- [ ] `cargo test -p argo-cli worker_actions_notify` passes.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-notify-fallback-risk-filter`
- PR title: `fix(notify): apply matches_risk in fallback dispatch (codex-notify-fallback-risk-filter) — F-05 P1`

## Notes for the implementer

- Approach (2) — fold risk matching into `dispatch_with_fallback` — is cleaner. Single ownership of the rule.
- This is a correctness fix, not a refactor. Keep the public API of `dispatch_with_fallback` stable; just add an optional `risk_level` parameter or extend the input struct.
- The existing `matches_risk` is the canonical predicate. Don't reinvent it.
