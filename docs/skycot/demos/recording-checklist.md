# v1.0 mission demo — recording checklist

Operator playbook for recording `docs/skycot/demos/v1-mission-mvp.mp4`.

The companion script is [`docs/skycot/demo_script.md`](../demo_script.md).
The dry-run gate is [`scripts/demo-dry-run.sh`](../../../scripts/demo-dry-run.sh).

## Pre-recording (T-1 day)

- [ ] **Dry-run is green.** `bash scripts/demo-dry-run.sh` exits 0.
      Inspect `${TMPDIR:-/tmp}/skycot-demo-dry-run/dry-run-results-*.json`
      and the per-scene screenshots — every row `status: "pass"`.
- [ ] **Throwaway provider key minted.** Generate a single-use
      Anthropic key with the lowest tier available; record the
      rotation date so it gets revoked within 24 h of recording.
- [ ] **Throwaway email mailbox open.** `demo@throwaway.cc` (or
      similar). Confirm the inbox receives a real magic-link by
      running the live signup flow once.
- [ ] **Slack workspace signed in.** Real workspace, SKYCOT app
      installed, target channel exists, operator's account visible in
      the channel member list.
- [ ] **Demo workspace seeded.** Empty database state; no prior
      workers; no prior audit rows. Reset with the operator's local
      reset script if needed.
- [ ] **Browser profile clean.** Use a fresh Chrome / Arc profile —
      no extensions, no autofill suggestions, no logged-in accounts.

## Pre-recording (T-0, 30 min before)

- [ ] **Laptop on AC power.** Battery >80% as a safety net.
- [ ] **OS do-not-disturb on.** macOS Focus → Do Not Disturb, all
      categories silenced.
- [ ] **Screen resolution.** Recording target is **1920×1080**. If
      using a 2560×1440 panel, set the recording region or use the
      OS's display-scaling preset to render at 1080p.
- [ ] **Font sizes / browser zoom.** Browser zoom 100%. Terminal font
      size legible at 1080p (≥14pt).
- [ ] **Terminal theme.** Dark theme with a high-contrast accent
      colour. No personal aliases visible in the prompt.
- [ ] **Mic test.** Record a 10-second sample, play it back. No room
      noise, no clipping on emphasised words.
- [ ] **Pre-warm caches.** Load `skycot.com` and the SPA `/onboarding/welcome`
      once in the recording browser — first-load LCP can break the
      "no mid-scene loads >2 s" constraint.
- [ ] **Notifications killed.** Slack desktop notifications off
      EXCEPT for the channel the demo uses. Mail off. Calendar off.

## During recording

- [ ] **Single take, no edits.** If something breaks mid-scene, stop
      and re-run the dry-run before attempting another take.
- [ ] **Talk-track timing.** Eye the scene boundaries against the
      target wall-clock (0:00, 0:30, 1:30, 3:30, 5:00, 6:00, 7:00).
      ±15 s drift per scene is acceptable; >30 s means re-take.
- [ ] **No key reveals.** The Anthropic key paste happens off-screen
      or with a partial-redacted screen recorder. The throwaway
      address is fine to show.
- [ ] **No live customer data.** Workspace name "Acme Co", worker
      name "Maya", any tool/MCP target points at sandbox accounts
      only.
- [ ] **Slack ✅ approval is clear.** The operator's click of the
      approve button on the Slack message must be visible — that's
      the core round-trip beat.
- [ ] **Audit hover lingers.** When the operator hovers a row group
      on `/audit` to show the hash-chain tooltip, hold for ≥2 s so
      viewers can read it.

## Post-recording

- [ ] **Trim to 5–7 min.** No edits inside the take; trim head and
      tail only.
- [ ] **Sanitization sweep.** Scrub every frame for:
  - [ ] Visible API keys / tokens.
  - [ ] Real email addresses (other than the throwaway).
  - [ ] Real Slack workspace member names other than the operator.
  - [ ] Browser autofill leaking other-site credentials.
- [ ] **Rotate the throwaway key.** Revoke the Anthropic key within
      24 h. Record the revocation timestamp in the workspace.
- [ ] **Save as `docs/skycot/demos/v1-mission-mvp.mp4`.** Confirm the
      file is committed by the operator (not by an agent — Git LFS
      may apply).
- [ ] **Upload as unlisted.** YouTube unlisted; share the link with
      the v1 review group. Do not flip to public until launch.
- [ ] **Update [`docs/build-plan/BUILD_STATUS.md`](../../build-plan/BUILD_STATUS.md)**
      row for `mission-v1-demo-recording` to MERGED.

## If a scene breaks mid-take

1. Stop recording.
2. Re-run `bash scripts/demo-dry-run.sh`. If it now fails, fix the
   underlying break first; the dry-run is the gate.
3. Reset workspace state (empty DB, no workers, no audit rows).
4. Reset the Slack channel (delete the prior test message).
5. New take from Scene 0.
