# V1.4 — Mobile approval app (Tauri)

**Owner**: claude. **Depends on**: D-push (#798),
H-orchestrator-routes (#827) merged.

## Scope

Operators want to approve / deny actions from their phone
without using a terminal. Existing PushApprovalChannel (#798)
sends APNs/FCM via Expo and includes a deep-link, but the
deep-link points at nothing — the mobile app doesn't exist
yet.

Ship a Tauri-based cross-platform mobile shell (iOS +
Android) with two screens:

1. **Pending approvals list** — fetches from
   `GET /approvals/pending?token=<bearer>` (existing route),
   renders one card per pending action with action_kind,
   worker name, payload preview, age, [Approve] [Deny]
   buttons.
2. **Settings** — bearer token entry (paste from
   `argo cred get APPROVAL_BEARER`), gateway URL, push
   notification permission toggle.

When a push notification opens the app, it deep-links straight
to the action card. One-tap approve/deny hits the existing
`/decisions/<token>?d=approve|deny` endpoint.

NO complex state, NO backend, NO sync — the app is a thin
client over the existing gateway routes.

## Files you may touch

NEW Tauri project at `mobile/argo-approve/`:
- `mobile/argo-approve/Cargo.toml` (NEW)
- `mobile/argo-approve/tauri.conf.json` (NEW)
- `mobile/argo-approve/src-tauri/src/main.rs` (NEW)
- `mobile/argo-approve/src/main.tsx` (NEW — minimal React/Solid;
  pick whatever Tauri's mobile-friendly default is in 2026)
- `mobile/argo-approve/src/screens/ApprovalsList.tsx` (NEW)
- `mobile/argo-approve/src/screens/Settings.tsx` (NEW)
- `mobile/argo-approve/src/api.ts` (NEW)
- `mobile/argo-approve/README.md` (NEW — install + run instructions)
- Workspace `Cargo.toml` — narrowly: add mobile/argo-approve
  to members IF Tauri integrates that way (otherwise standalone)

## Acceptance criteria
- [ ] `tauri ios dev` and `tauri android dev` both build + run
- [ ] Approvals list fetches from real gateway + renders
- [ ] Approve/Deny tap hits real endpoint + updates list
- [ ] Push deep-link opens to specific approval (URI scheme:
      `argo://approve/<token>`)
- [ ] Settings persists bearer + gateway URL via Tauri's
      secure storage API
- [ ] README has 5-step setup instructions for new operator
- [ ] No external CDN deps — everything bundled
- [ ] App size under 30 MB

## Branch + PR
- Branch: `claude-mobile-approval-app-tauri`
- PR title: `mobile: Tauri approval app — iOS + Android shell (claude-mobile-approval-app-tauri)`
