# V1.4 — Mobile approval deep-link improvements

**Owner**: claude.

**Authored against**: origin/main 2026-05-09 (split-note v2 by conductor — original v1 bundled deep-link handler code refactor + on-device manual QA into one brief. The QA gate cannot be satisfied by current workers (no mobile-build environment). Split: this brief stays BLOCKED on device-QA; the code-only prep portion lands via NEW sibling `claude-approval-mobile-deep-link-prep`). **Depends on**: claude-mobile-approval-app-tauri
(#943) merged.

## Why

The Tauri-based `mobile/argo-approve` shell shipped in #943 is
a thin one-card-at-a-time client. Real operators on the road
need: bulk approve when 10 cards stack up, push notification
grouping (so a swarm doesn't generate 30 separate banners),
snooze (defer for 15m without losing the request), and a
biometric guard before the decision actually fires (FaceID /
TouchID / Android BiometricPrompt). Mission claim 6's
multi-channel ambition extends to mobile UX, not just protocol
support.

## Scope

Four user-facing additions, each independent:

1. **Bulk approve** — multi-select on the pending list, single
   `[Approve N]` / `[Deny N]` action.
2. **Push notification grouping** — APNs / FCM thread-id keyed
   on `worker_id` so a 30-card swarm collapses to one banner
   that expands to the list.
3. **Snooze** — per-card `[Snooze 15m]` button. Local state
   only (no gateway change); the card vanishes from the
   pending list for 15m, then reappears. If the underlying
   approval is decided server-side in the meantime, snooze is
   discarded.
4. **Biometric guard** — before the actual `/decisions/<token>?d=approve`
   POST fires, prompt FaceID / TouchID. Configurable per-decision
   (always / approve-only / off) in Settings. Default: approve-only.

NOT in scope: changes to the gateway endpoints — every feature
is client-side. NOT a redesign of the existing two-screen UX.

## Approach

1. Bulk approve — `mobile/argo-approve/src/screens/ApprovalsList.tsx`
   gains a selection state + a footer action bar. POSTs are
   issued sequentially with a small concurrency cap (3) so
   we don't hammer the gateway.
2. Push grouping — `mobile/argo-approve/src-tauri/src/push.rs`
   sets the `apns-thread-id` header (iOS) or
   `android.notification.tag` (Android) to the worker id. No
   server change required.
3. Snooze — `mobile/argo-approve/src/snooze.ts` keeps a
   local `Map<token, deadline>` in Tauri secure storage. The
   list filter excludes snoozed-and-not-yet-expired tokens.
4. Biometric — `mobile/argo-approve/src-tauri/src/biometric.rs`
   wraps `tauri-plugin-biometric` (or equivalent 2026 plugin).
   Settings screen gains a tri-state toggle.

## Files you may touch

NEW or extend within the existing `mobile/argo-approve/` tree:
- `mobile/argo-approve/src/screens/ApprovalsList.tsx` — bulk
  selection UI
- `mobile/argo-approve/src/screens/Settings.tsx` — biometric
  toggle + snooze defaults
- `mobile/argo-approve/src/snooze.ts` (NEW)
- `mobile/argo-approve/src/api.ts` — narrowly: bulk dispatch
  helper (3-at-a-time)
- `mobile/argo-approve/src-tauri/src/push.rs` (NEW or extend)
- `mobile/argo-approve/src-tauri/src/biometric.rs` (NEW)
- `mobile/argo-approve/src-tauri/Cargo.toml` — narrowly: add
  biometric plugin (verify against project_argo_dep_allowlists.md;
  mobile is its own tree, allowlist may not apply but cite it
  in the PR)
- `mobile/argo-approve/README.md` — narrowly: document the new
  features + setup notes

## Files you must NOT touch

- Gateway routes — every feature is client-side
- `crates/argo-cli/` — no Rust workspace change
- The bearer / token exchange protocol
- Existing two-screen architecture (no third screen)

## Acceptance criteria

- [ ] Selecting 5 cards and tapping `[Approve 5]` issues 5
      decisions and updates the list (manual QA on iOS sim +
      Android emulator)
- [ ] Push notifications from the same worker_id collapse
      into one expandable thread on iOS 17+ and Android 14+
- [ ] `[Snooze 15m]` removes the card; after 15m it
      reappears with a "snoozed earlier" subtitle
- [ ] If the approval is decided server-side during snooze,
      it disappears from the list cleanly (no zombie row)
- [ ] Biometric guard prompts on approve when set to
      `approve-only`; cancelling the prompt aborts the POST
- [ ] App size still under 30 MB (ceiling from #943)
- [ ] `tauri ios dev` + `tauri android dev` both build clean
- [ ] README updated with screenshots / setup steps

## Branch + PR

- Branch: `claude-approval-mobile-deep-link-improvements`
- PR title: `mobile: bulk + grouping + snooze + biometric (claude-approval-mobile-deep-link-improvements)`

## Notes for the implementer

- Bulk POSTs MUST be sequential-with-cap, not full parallel —
  the gateway has its own per-channel rate limit (see
  `claude-approval-rate-limit`). 3-at-a-time is the safe ceiling.
- Snooze is purely local state — DO NOT add a snooze field to
  the server-side approval row. Keeps the brief small and
  avoids a migration.
- For biometric, fail-closed: if the platform plugin returns
  an error, treat it as a cancelled decision. Never silently
  bypass the prompt.
- Push grouping: the `worker_id` is in the existing push
  payload; just set the platform-level thread/tag header from
  it. Don't change the payload shape.
