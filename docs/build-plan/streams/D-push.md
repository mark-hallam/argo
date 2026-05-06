# Stream D — Push approval channel (Expo)

**Owner**: codex. **Depends on**: D-email merged.

## Scope

`PushApprovalChannel` impl. Sends a native push notification
to the operator's mobile device (iOS via APNs, Android via FCM
— Expo abstracts both). Tapping the notification deep-links
into the SKYCOT mobile inbox (Stream D-webinbox covers the web
inbox; mobile app is a separate v1.x deliverable, but the push
infrastructure lands now).

## Files you may touch
- `crates/argo-channels/src/lib.rs` — narrowly scoped: add ONE
  line `pub mod push;` (or `mod push;`). No other edits.
- `crates/argo-channels/src/push.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs`
  (`POST /channels/push/devices` for device registration;
  `POST /channels/push/decision/<token>` for in-app settle)
- `manifests/channels/push.toml`

## Files you must NOT touch
- Other channel impls
- `NotifyChannel` trait

## Acceptance criteria
- [ ] `PushApprovalChannel` impl of `NotifyChannel`
- [ ] Expo push API client (`expo_server_sdk` or thin reqwest
      calls)
- [ ] Device registration flow: `POST /channels/push/devices`
      with `{ operator_id, expo_push_token, platform }`;
      stored per-operator
- [ ] Notification body: action summary, tap deep-links to
      `argo://inbox/approval/<token>` (the schema the future
      mobile app will handle; for now the URL is informational
      + the gateway also accepts `/channels/push/decision/<token>?d=approve|reject`)
- [ ] Integration test gated on `EXPO_TEST_TOKEN`
- [ ] Clippy clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-D-push`
- PR title: `channels: push approval channel via Expo (APNs/FCM) (D-push)`
