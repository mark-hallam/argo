# mission-quiet-hours-channels

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `ba94edd63` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-quiet-hours-channels`.

🟡 **P2 MISSION-CLAIM-6 (Multi-channel approvals).** Source: `docs/build-plan/MISSION_BACKLOG.md` item #121:

> "Add quiet hours for SMS/Voice/WhatsApp/iMessage/Push/Slack."

Enterprise customers (and operators with sleep schedules) want noisy interrupt channels (SMS, Voice, WhatsApp, push notifications) gated by quiet-hours windows. Inside the window, the dispatcher should either:
- Skip the noisy channel and fall through to the next entry in the fallback chain (e.g. SMS → WebInbox after 10pm), OR
- Queue the notification for delivery at the next non-quiet window (more complex; defer to v1.x).

This brief implements (1) — skip noisy channels in quiet hours, fall through to the next non-noisy channel.

## Scope

### Configuration shape

Add to `argo.toml`:

```toml
[operator.quiet_hours]
enabled = true
start = "22:00"   # HH:MM in operator's local TZ
end   = "07:00"   # wraps midnight if end < start
timezone = "Australia/Melbourne"  # IANA TZ
noisy_channels = ["sms", "voice", "whatsapp", "imessage", "push"]
# slack is excluded by default — slack DMs aren't noisy if user has DND on at OS level
```

If absent or `enabled = false`, behavior is unchanged from current main.

### Resolver

Extend `OperatorPreferencesResolver` (or sibling) with `is_quiet_now(channel, now: DateTime<Utc>) -> bool`:
1. Parse `start` + `end` + `timezone` into a `chrono_tz::Tz` window.
2. Convert `now` to operator's TZ.
3. Check if (a) the window matches AND (b) the channel is in `noisy_channels` → return `true`.
4. Otherwise `false`.

### Dispatch integration

In `worker_actions_notify.rs::try_dispatch_on_ask` (or wherever the FallbackChain is iterated), after `promote_preferred` (#2059), filter the chain:

```rust
let now = chrono::Utc::now();
let chain = chain.into_iter()
    .filter(|ch| !quiet_hours_resolver.is_quiet_now(ch, now))
    .collect();
```

If filtering empties the chain (all channels are quiet), fall through to `WebInbox` as the always-on default (or log a warning + still send via the original chain — operator-decisional, document the choice).

### Tests

Sibling `_tests.rs` covers:
- `quiet_hours_skips_sms_during_window` — 23:00 in Melbourne TZ → SMS skipped, WebInbox dispatched.
- `quiet_hours_no_effect_outside_window` — 12:00 in Melbourne TZ → SMS dispatched.
- `quiet_hours_window_wraps_midnight_correctly` — start=22:00 end=07:00, check at 02:00 → in window.
- `quiet_hours_disabled_preserves_chain` — `enabled = false` → no filtering.
- `quiet_hours_empty_chain_falls_through_to_webinbox` — all 5 noisy channels in chain at 02:00 → result is `[WebInbox]`.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-config/src/sections.rs` (or sections_operator.rs) — narrowly scoped: add `[operator.quiet_hours]` parsing.
- `crates/argo-channels/src/quiet_hours.rs` — NEW: ~80 LOC `QuietHoursResolver`.
- `crates/argo-channels/src/quiet_hours_tests.rs` — sibling tests ~120 LOC.
- `crates/argo-cli/src/handlers/worker_actions_notify.rs` — narrowly scoped: add the filter step (~10 LOC) after the existing `promote_preferred` call.
- `docs/skycot/operator/quiet-hours-config.md` — NEW: ~60 LOC operator doc.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Existing `OperatorPreferencesResolver` — extend via sibling, don't refactor.
- The channel manifests / channel sender implementations.

## Acceptance criteria

- [ ] `QuietHoursResolver::is_quiet_now(channel, now)` exists with correct DST/wrap-midnight handling (`chrono_tz`).
- [ ] All 5 tests pass.
- [ ] `try_dispatch_on_ask` filters the chain via quiet-hours after `promote_preferred`.
- [ ] Empty-chain fallback to `WebInbox` documented + tested.
- [ ] `argo.toml` config section parsed cleanly (round-trip serde test).
- [ ] Operator doc covers: window definition, timezone, noisy_channels list, empty-chain behavior.
- [ ] `cargo clippy -p argo-channels -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-quiet-hours-channels`
- PR title: `feat(channels): quiet-hours filter for noisy notify channels (mission-quiet-hours-channels)`

## Notes for the implementer

- ~250-350 LOC expected. Resolver ~80, tests ~120, config parsing ~30, dispatch integration ~15, doc ~60.
- **`chrono_tz` is the right TZ crate** — already a workspace dep. Don't pull in another.
- **DST handling matters**: Australia/Melbourne switches between AEST/AEDT. The window 22:00-07:00 in local time should remain 22:00-07:00 across the switch (don't pin to UTC offsets).
- **The empty-chain fallback is operator-decisional**. Default to `WebInbox` (always-on); document it; future flag in config if operators want different behavior.
- This brief stacks naturally on #2059 (`mission-channels-operator-preferred-fallback`). Filter happens AFTER promote_preferred.
- Reference: mission backlog item #121.
