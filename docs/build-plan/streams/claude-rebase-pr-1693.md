# claude-rebase-pr-1693 — Rebase #1693 claude-tui-approval-tray onto current main

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main HEAD on 2026-05-15.

## Scope

PR #1693 (claude-tui-approval-tray) went DIRTY when sister TUI PRs #1692 (keybindings-help) + #1694 (spend-monitor) merged ahead of it. Conflicts are on shared TUI files (input_globals.rs, overlays.rs, app.rs) where each PR adds a new variant/registration.

## Files you may touch

- All files in original PR #1693
- Standard rebase merge: combine the input_globals/overlays/app additions with the now-merged ones (additive — each PR adds a different variant to the registry).

## Acceptance criteria

- [ ] PR #1693 shows MERGEABLE / CLEAN after force-push.
- [ ] All 3 TUI overlay registrations coexist in input_globals + overlays.
- [ ] cargo test -p argo-tui passes.

## Branch + PR

- Branch (existing): `codex-claude-tui-approval-tray`
- PR (existing): #1693

## Notes

Standard rebase: `gh pr checkout 1693 && git fetch origin main && git rebase origin/main`. Additive merges on input_globals.rs (3 variants now), overlays.rs (3 draw_ fns now), app.rs (3 state fields now). `git push --force-with-lease`.
