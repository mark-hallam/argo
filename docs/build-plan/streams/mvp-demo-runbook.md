# MVP demo runbook — operator one-pager for recording v1 demo

**Owner**: claude. **Depends on**: BrowserExecutor MERGED #1247; Slack approval roundtrip MERGED #1229; existing `docs/skycot/demo_script.md` SHIPPED; gateway boot SHIPPED.
**Authored against**: origin/main `0967cb5d` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mvp-demo-runbook`.

> SOURCE: gap surfaced by MVP-readiness investigation 2026-05-11. The v1 demo requires non-trivial operator setup (Slack app + signing secret + ngrok tunnel + BrowserExecutor-enabled vertical + provider key). Nothing in `docs/skycot/runbook/` walks an operator through that today. This brief authors a one-pager that lists every command + every env var + every external account setup in sequence, so the operator can prep + record without back-and-forth.

## Scope

Author NEW `docs/skycot/runbook/v1-demo-recording.md` with these sections (target ≤ 350 LOC):

1. **What you'll record**: 1-paragraph high-level outline of the 5-7 min flow (per `docs/skycot/demo_script.md` plus BrowserExecutor + Slack subs).

2. **Pre-flight checklist** (before opening the recorder):
   - ✅ Rust toolchain installed; repo on origin/main
   - ✅ `argo` binary built (`cargo build --release -p argo-cli`)
   - ✅ `ANTHROPIC_API_KEY` exported (NOT in worker context — use a separate terminal)
   - ✅ ngrok / cloudflared installed
   - ✅ Slack app created (with manifest below) + signing secret captured
   - ✅ Python 3 on PATH (required for `browser_mock_mcp.py` if using mock browser flow)
   - ✅ `marketplace-seller-browser` vertical exists (depends on `mvp-demo-vertical-browser-marketplace` MERGED — note as dependency if not yet shipped)

3. **Slack app setup** (5-10 min):
   - Step-by-step: create app at api.slack.com, paste the included App Manifest YAML, capture signing secret + bot token, configure interactivity URL placeholder.
   - List the **exact OAuth scopes** needed: `chat:write`, `incoming-webhook`, `commands` (if using slash commands), plus inbound interactivity endpoint.
   - Include a known-good App Manifest snippet (operator copies into Slack's create-from-manifest flow).

4. **Tunnel setup** (2 min):
   - `ngrok http 7400` or `cloudflared tunnel run --url http://localhost:7400`
   - Paste public URL into Slack app's Interactivity URL (`<public>/channels/slack/webhook`)
   - Document that the URL must START WITH HTTPS and END WITH the exact path

5. **Environment variables**:
   - `ANTHROPIC_API_KEY` = real key (paid tier with credit)
   - `SLACK_APPROVALS_SIGNING_SECRET` = the v0 signing secret from Slack app config
   - `SLACK_WEBHOOK_URL` = the outbound webhook (for posting Asks)
   - `ARGO_BROWSER_MCP_BIN` = path to browser MCP server binary (use `tests/fixtures/browser_mock_mcp.py` for mock)
   - `ARGO_GATEWAY_AUTH_BYPASS=1` (skip bearer auth for the demo session)

6. **Recording session** (the actual demo, 5-7 min):
   - Exact sequence of `argo` commands, copy-paste ready, ordered for screen recording
   - Each command's expected output (so operator knows when something failed)
   - Pauses for narration noted

7. **Post-recording**:
   - Output paths: `docs/skycot/demos/v1-mission-mvp.mp4` + `docs/skycot/demos/v1-mission-mvp.md` transcript
   - Commit message template
   - PR checklist for the v1 demo brief sign-off

This is **a doc-only brief** — NO code changes, NO new tests, NO migrations. Pure operator-facing runbook.

This is NOT in scope: building the recording (operator-only); rebuilding the Slack app from scratch (uses existing manifest pattern); writing a recording wrapper script (that's a separate brief if needed).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `docs/skycot/runbook/v1-demo-recording.md` (NEW, ≤ 350 LOC) — the runbook itself
- `docs/skycot/runbook/README.md` (if it exists — verify in first 5min) — narrowly scoped: ADD a one-line link to the new runbook

## Files you must NOT touch

- `docs/skycot/demo_script.md` — the script itself stays untouched; runbook references it
- Any `crates/` source
- Any `verticals/` directory
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `docs/skycot/runbook/v1-demo-recording.md` exists with all 7 sections
- [ ] Pre-flight checklist has copy-paste shell verifications for each item
- [ ] Slack app setup includes a working App Manifest YAML snippet
- [ ] All env vars listed with example values + where to get them
- [ ] Recording session section has the exact CLI command sequence in order
- [ ] Each CLI command shows expected output snippet
- [ ] Post-recording section includes commit message + transcript skeleton
- [ ] No code changes, no tests, no migrations
- [ ] `docs/skycot/runbook/README.md` (if exists) cross-references the new file
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mvp-demo-runbook`
- PR title: `docs: v1 demo recording operator runbook (mvp-demo-runbook)`

## Notes for the implementer

- This brief delivers **operator value**, not engine value — the goal is "one operator-readable doc that takes a clean machine to a recording-ready setup in 30 minutes."
- Don't be over-comprehensive. Skip troubleshooting sections, FAQ, alternative paths. ONE happy path, deeply documented.
- For the Slack App Manifest: use the minimal viable shape that works with `/channels/slack/webhook` and the existing `slack_inbound.rs` HMAC verification. Test it against the existing `slack_approval_roundtrip_e2e` if you can.
- For the recording session sequence: mirror `docs/skycot/demo_script.md` but substitute BrowserExecutor for HumanExecutor at the `post_listing` step, and Slack-approval for `argo worker actions approve`.
- The `mvp-demo-vertical-browser-marketplace` brief is a sibling dependency — note that in section 2.
