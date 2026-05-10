# mission-v1-demo-recording — v1.0 ship-gate demo recording

**Owner**: claude. **Depends on**: mission-wire-browser-action-executor (M2, hard), mission-wire-slack-approval-roundtrip (M3, hard); ideally mission-routing-skillmatch-rolematch (M4), mission-genesis-end-to-end-smoke (M5), mission-skill-propagation-loop (M6), mission-auto-approval-budget-e2e (M7) for the most compelling demo. **Tier M — Mission MVP critical path. v1.0 ship gate per BUILD_PLAN_2WK.md.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Mission claim: SKYCOT v1 ships when the mission MVP demo runs end-to-end against the
real binary, recorded as a single take. This is the hard ship gate per
`BUILD_PLAN_2WK.md`. The demo replaces HumanExecutor with BrowserExecutor for at least
one approved action and replaces the CLI approval path with Slack for at least one
approval — proving the claims that are differentiating versus competitors. No source
code in this brief; it is the recording + transcript.

## Approach (concrete)

1. **Read** the existing `docs/skycot/demo_script.md` end-to-end. Identify the steps
   currently using HumanExecutor and the steps currently using the CLI approval path.
2. **Minor edits** to `docs/skycot/demo_script.md` only if the script needs updating
   to call BrowserExecutor (M2) or Slack inbound webhook (M3). Keep the diff small;
   the substantive change is the recording, not the script.
3. **Record** the demo end-to-end against a clean tmpdir `ARGO_HOME` using the real
   `argo` binary built from `origin/main`:
   - At least one action runs through BrowserExecutor (M2) — capture the browser
     window in the recording.
   - At least one approval runs through Slack inbound (M3) — capture the Slack
     message + click in the recording.
   - If M4 is merged: include the cross-worker handoff segment.
   - If M5 is merged: open with `argo genesis "..."` and worker-create.
   - If M6 is merged: include a brief skill-propagation callout.
   - If M7 is merged: include an auto-approval-vs-Ask callout.
4. **Save artifacts**:
   - `docs/skycot/demos/v1-mission-mvp.mp4` — the screen recording (target
     ≤ 10 minutes; aim for a tight 5-7 minute take).
   - `docs/skycot/demos/v1-mission-mvp.md` — the transcript: a step-by-step written
     walkthrough mirroring the recording, with each segment timestamped, the exact
     CLI commands used, and screenshot stills at key moments.
5. **No source code changes** beyond the optional small edits to `demo_script.md`.

## Files you may touch

- `docs/skycot/demos/v1-mission-mvp.md` (NEW transcript)
- `docs/skycot/demos/v1-mission-mvp.mp4` (NEW recording artifact)
- `docs/skycot/demos/v1-mission-mvp/` (NEW directory) — supporting screenshot stills
  if the transcript embeds them.
- `docs/skycot/demo_script.md` — minor edits ONLY if the script requires updates to
  call BrowserExecutor + Slack instead of HumanExecutor + CLI.

## Files you must NOT touch

- Any source code under `crates/`.
- Any other doc under `docs/` except the demo script and the new demo directory.
- BUILD_STATUS.md (conductor only).

## Acceptance criteria

- [ ] M2 (mission-wire-browser-action-executor) is MERGED on `origin/main` at recording time.
- [ ] M3 (mission-wire-slack-approval-roundtrip) is MERGED on `origin/main` at recording time.
- [ ] At least one action in the recording runs through BrowserExecutor.
- [ ] At least one approval in the recording runs through Slack (not CLI).
- [ ] `docs/skycot/demos/v1-mission-mvp.mp4` exists and plays end-to-end (≤ 10 min).
- [ ] `docs/skycot/demos/v1-mission-mvp.md` transcript mirrors the recording with
      timestamps + exact CLI commands.
- [ ] PR description names which optional dependencies (M4-M7) were included.
- [ ] No source-code changes outside the optional `demo_script.md` minor edits.

## Branch + PR

- Branch: `claude-mission-v1-demo-recording`
- PR title: `demo: v1.0 mission-MVP end-to-end recording + transcript (mission-v1-demo-recording)`

## Notes for the implementer

- This is the v1.0 ship gate. If M2 or M3 are not yet MERGED at claim time, post a
  BLOCKED note on this brief naming the missing dependency and pick a different brief.
- For the recording, prefer macOS `screencapture -v` or QuickTime screen recording;
  keep the resolution at 1080p or above. mp4 (H.264) is the target format.
- Trim aggressively in post — silence between commands, long compile waits, etc.
  Target a tight 5-7 minute final cut.
