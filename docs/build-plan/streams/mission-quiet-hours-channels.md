# Mission load-bearing wire — generalize quiet_hours from email to all NotifyChannels

**Owner**: claude. **Depends on**: `argo-channels::email::QuietHours` exists (verified at `crates/argo-channels/src/email.rs:214`); `NotifyChannel` trait surface stable.
**Authored against**: origin/main `cb0c0a90` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-quiet-hours-channels`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 item #121 ("Add quiet hours for SMS/Voice/WhatsApp/iMessage/Push/Slack"). Today `QuietHours` is a struct on email config only (`crates/argo-channels/src/email.rs:214`); when an email send falls inside the quiet window, the channel returns a `deferred-send: quiet_hours` outcome. The other 7 NotifyChannel implementations have no quiet-hours awareness — meaning a 3am Slack ping during quiet hours sails through. Mission Claim 6 ("multi-channel approvals") promises operator-respectful escalation; that requires the quiet-hours discipline to be channel-trait-level, not email-specific.

## Scope

Lift the `QuietHours` type out of `email.rs` into a NEW shared module `crates/argo-channels/src/quiet_hours.rs` (≤ 150 LOC) and wire it into the `NotifyChannel` trait surface as a `pub fn quiet_hours(&self) -> Option<&QuietHours>` accessor with a default returning None (so today's non-email channels compile unchanged). Per-channel configs gain a `quiet_hours: Option<QuietHours>` field; the trait's `send` implementation runs the quiet-hours check before calling the channel-specific dispatch path. When the check defers, the channel returns the existing `deferred-send: quiet_hours` outcome shape.

Channels updated (config field + send-time check):
- `slack.rs`, `sms.rs`, `voice.rs`, `whatsapp.rs`, `imessage.rs`, `push.rs`, `webinbox.rs` (7 channels)
- `email.rs` migrates from owning `QuietHours` to importing the shared module (back-compat: existing field name preserved)

This is NOT in scope: per-recipient quiet hours (only per-channel for v1); operator timezone resolution (`QuietHours::contains` already handles UTC + TZ math today — reuse, don't redesign); UI for editing quiet hours (CLI / config-file only); per-action overrides.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-channels/src/quiet_hours.rs` (NEW, ≤ 150 LOC) — lifts `QuietHours` struct, `Default`, `contains`, and any helpers from `email.rs`. Adds doc-comment with Slack/SMS examples.
- `crates/argo-channels/src/lib.rs` — narrowly scoped: ONE `pub mod quiet_hours;` line + ONE `pub use quiet_hours::QuietHours;` re-export. NO other edits.
- `crates/argo-channels/src/email.rs` — narrowly scoped: REMOVE the local `QuietHours` struct definition + impls; replace with `use crate::quiet_hours::QuietHours;`. The `quiet_hours: QuietHours` field on email config stays. Body code unchanged.
- `crates/argo-channels/src/slack.rs` — ADD `quiet_hours: Option<QuietHours>` to the channel's config struct + `Default::default()`; ADD a check at the top of `send` that returns the deferred outcome on hit.
- `crates/argo-channels/src/sms.rs` — same pattern.
- `crates/argo-channels/src/voice.rs` — same pattern.
- `crates/argo-channels/src/whatsapp.rs` — same pattern.
- `crates/argo-channels/src/imessage.rs` — same pattern.
- `crates/argo-channels/src/push.rs` — same pattern.
- `crates/argo-channels/src/webinbox.rs` — same pattern.
- `crates/argo-channels/src/quiet_hours_tests.rs` (NEW, sibling) — 8 tests: lifted-from-email-tests-pass; one each per channel (slack-quiet-hours-defers, sms-quiet-hours-defers, etc.).
- `crates/argo-channels/Cargo.toml` — narrowly scoped: confirm `chrono` is present (it is, used by current QuietHours). NO other Cargo edits.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/policy_handlers.rs` — already references `quiet_hours` (line 120-121) but for `rule.quiet_hours`, which is policy-level, NOT channel-level — orthogonal concern, do not unify
- Any non-channels crate
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `QuietHours` lives in `crates/argo-channels/src/quiet_hours.rs` (NEW); re-exported from `argo_channels::QuietHours`
- [ ] `email.rs` no longer defines its own `QuietHours`; the email config's field remains the same shape
- [ ] All 7 non-email channels carry `quiet_hours: Option<QuietHours>` in their config struct, defaulting to None (today's behavior preserved)
- [ ] Each non-email channel's `send` returns the deferred-send shape when quiet hours match, before attempting actual delivery
- [ ] The deferred outcome `kind` matches the existing email pattern (`"quiet_hours"` reason string verbatim — searchable across telemetry)
- [ ] No regression in email behavior: the existing 4 quiet-hours email tests continue to pass with their assertions intact
- [ ] 8 new tests in `quiet_hours_tests.rs` exercise the trait-level pattern + each channel
- [ ] No new public API on `NotifyChannel` trait beyond the optional `quiet_hours()` accessor
- [ ] `cargo build -p argo-channels` clean
- [ ] `cargo clippy -p argo-channels --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-channels` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-quiet-hours-channels`
- PR title: `channels: generalize quiet_hours from email to all NotifyChannels (mission-quiet-hours-channels)`

## Notes for the implementer

- The lift is mechanical. Open `email.rs` lines 214-260ish, copy `QuietHours` + impls + tests verbatim into the new module, then refactor email.rs to import from the new module.
- For the trait-level check, prefer extending the trait with a `pub fn quiet_hours(&self) -> Option<&QuietHours> { None }` default method, then have `send` call it (also as a default method, factored into a `defer_for_quiet_hours_or<F, Fut>(&self, fut: F) -> ...` helper). Per-channel configs override `quiet_hours()` to return their field; per-channel `send` overrides only when the channel needs custom dispatch logic.
- Each channel's existing `send` implementation needs the quiet-hours check inserted BEFORE the actual delivery attempt. The cheapest pattern is: extract the body of `send` into a `send_now` private fn, then have `send` do `if quiet { return deferred; } else { send_now(...) }`.
- The `Default` impl for `Option<QuietHours>` on the new channel configs MUST be `None`, NOT `Some(QuietHours::default())` — defaulting to "no quiet hours" preserves today's behavior.
- The `deferred_reason: Some("quiet_hours".into())` string must be byte-identical across channels — operators grep for that exact reason in audit logs.
- If you find that a channel's `send` is non-async or returns a different outcome shape than the email pattern, scope the v1 brief to JUST those channels that fit the pattern and BLOCK on `mission-channel-trait-unification` for the outliers — do not paper over heterogeneous shapes inside this brief.
