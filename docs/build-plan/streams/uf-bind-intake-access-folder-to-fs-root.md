# uf-bind-intake-access-folder-to-fs-root

**Owner**: claude or codex — Genesis hire path; conductor review before merge.
**Depends on**: fs_root-at-hire shipped (`cf2b199`, store_hire_fs_root.rs).
**Authored against**: origin/main `5e73d548b` on `2026-06-11`.
**Validate before claim**: bash scripts/check-brief-freshness.sh uf-bind-intake-access-folder-to-fs-root

## Problem (found live in the 2026-06-11 E2E stranger walkthrough)

The Genesis intake CAPTURES the user's working folder — in the walkthrough the
user said "the invoice files in the Inbox folder inside SkycotDemo on my Mac"
and the intake surfaced it as **Access needed (draft-only): "SkycotDemo/Inbox
folder"** in the captured panel. But at hire that folder is **dropped**:
`store_hire_fs_root::resolved_fs_root` (shipped 2026-06-11) sets `worker.fs_root`
to a generated default `<HOME>/SkycotTeammates/<persona-slug>/`, NOT the folder
the user named. So a teammate whose whole job is "read the files in MY folder"
is pointed at an empty generated folder instead. The operator explicitly named
this as remaining work ("binding the folder you named in intake to the
teammate's working folder — today it gets a sensible default"). Evidence:
`docs/skycot/audit/2026-06-11-e2e-stranger-walkthrough.md` (beat 5 + fs_root
follow-up note).

## Goal

When the intake's Access slot captures a concrete local folder path, thread it
to `worker.fs_root` at hire (preferring it over the generated default), so the
teammate works in the folder the user actually named.

## Scope / approach

1. **Carry the Access folder through the hire draft.** The intake Access slot
   value (`IntakeSlot::Access`) and/or the captured `draft_integrations` already
   record "SkycotDemo/Inbox folder". Find where the hire draft is assembled
   (`web/lib/api/genesis-hire-intake.ts` `buildHireDraftFromGenesis` +
   `crates/argo-identity/src/store_hire.rs` `HireDraft`) and carry a resolved
   filesystem path when the Access answer denotes one.
2. **Resolution + safety.** Only bind a path that resolves to a real directory
   under the user's HOME (expand `~`, reject `..`/traversal, reject paths
   outside HOME — reuse the `with_fs_root` guard that already rejects empty/`..`).
   If the Access answer is vague / not a path, KEEP the generated default
   (`resolved_fs_root` precedence: explicit-draft-path > captured-Access-path >
   Genesis-default > None). Fail-soft: never block the hire on a bad path.
3. **Surface it.** The hire review screen / done page should state the working
   folder so the user can confirm it (truthful, no overclaim).

## Acceptance

- Intake naming a real folder under HOME → `worker.fs_root` = that folder
  (verified by a hire test asserting the column); a vague Access answer →
  unchanged generated default (regression pin); a traversal/outside-HOME path →
  rejected, falls back to default (security pin).
- The capability fence's `FsRead{<fs_root>/**}` then scopes to the user's real
  folder — the teammate reads the files it was hired to read.
- Files < 500 LOC; the path-resolution stays in the store/gateway layer (no fs
  side effects in argo-identity beyond computing the path; mkdir stays in the
  hire route, fail-soft).

## Parallel-surface check

`rg 'fs_root' crates/argo-identity crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs`
— the resolver is `store_hire_fs_root::resolved_fs_root`; extend it, do not fork.
