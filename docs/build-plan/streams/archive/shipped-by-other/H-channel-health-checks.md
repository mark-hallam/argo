# Stream H (follow-up) — `argo doctor` channel health pings

**Owner**: codex. **Depends on**: H-doctor-extensions (#808)
merged + at least one channel impl from D-* merged (all 4
are MERGED).

## Scope

Extend `argo doctor` with a new check section that pings
each configured `NotifyChannel` for connectivity. Doesn't
SEND a real notification — just probes the channel's
liveness:

- email (Postmark): hit the Postmark `/server` endpoint
  with the configured server token
- slack: `auth.test` API call with the bot token
- sms (Twilio): `Accounts.json` GET with the account SID
- push (Expo): `https://exp.host/--/api/v2/server-info`

Each check returns `OK` (200), `WARN` (rate-limited or
slow), or `ERROR` (4xx / 5xx / network failure). Output
extends the existing `Channels:` section in `argo doctor`.

## Files you may touch
- `crates/argo-cli/src/handlers/doctor_channels.rs` (NEW)
- `crates/argo-cli/src/handlers/doctor.rs` — narrowly:
  ONE call into the new module; do NOT modify other checks
- `crates/argo-cli/Cargo.toml` — narrowly: add `reqwest`
  if not already there

## Files you must NOT touch
- `crates/argo-channels/**` — read-only consumer
- `crates/argo-cli/src/handlers/doctor_h_stack.rs` (from
  H-doctor-extensions) — that's a sibling check; this brief
  is a NEW sibling
- Any other doctor check

## Acceptance criteria
- [ ] 4 channel pings implemented; each timeouts after 5s
- [ ] Missing config (e.g. no Postmark token) reports
      `MISSING` not `ERROR`
- [ ] Rate-limited responses (429) report `WARN` with the
      retry-after value
- [ ] `argo doctor --json` exposes the per-channel
      sub-results
- [ ] Network failures don't crash; report `ERROR` with the
      error text
- [ ] Tests for each path using a mock HTTP server
      (`mockito` or similar) — gated unit tests
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-channel-health-checks`
- PR title: `doctor: NotifyChannel connectivity pings (H-channel-health-checks)`

## Notes
- This is the "is the platform healthy?" question for
  customer support: an operator can't get approval
  notifications → they run `argo doctor` and immediately
  see WHICH channel is unreachable.
- All channel pings are HEAD/GET requests to known liveness
  endpoints — never POST. Don't accidentally send a real
  notification.
