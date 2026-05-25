# SKYCOT v1 demo — operator machine setup runbook

> **Canonical scene-by-scene narration lives in
> [`../demos/v1-mission-mvp-runbook.md`](../demos/v1-mission-mvp-runbook.md).**
> This doc has been narrowed to the machine setup half — Slack app,
> tunnel, env vars, SQLCipher key, demo-mode env vars. The six-scene
> talk track and timings are owned by the canonical runbook and are
> not duplicated here.

Trust register reviewed: 2026-05-24 (operator-locked-soften per wave-3 audit + PR #2125 + this consolidation pass).

Goal: take a clean machine to a recording-ready setup in ~30 minutes,
then narrate the canonical eight-minute v1 mission demo from
[`../demos/v1-mission-mvp-runbook.md`](../demos/v1-mission-mvp-runbook.md).
**One happy path** — troubleshooting, FAQ, and alternative paths are
out of scope.

The §1 marketplace-seller-browser flow below pre-dates the six-claim
mission structure. Use it as a worked example for env-var wiring and
Slack-approval round-trip ergonomics; the actual on-camera narration
follows the canonical runbook's six scenes.

---

## 1. What you'll record

A 5–7 minute screen capture of the marketplace-seller vertical
running end-to-end with the v1 stack:

- Operator drops a one-line brief (`argo work add`).
- Worker runs (`argo work serve --once`), produces research, pricing,
  draft listings, and an approval-gated `post_listing` action.
- Outbound notify posts the action to Slack; operator clicks **Approve**;
  inbound webhook verifies HMAC and flips state to `approved`.
- `argo worker actions execute <id>` drives the BrowserExecutor against
  a mock browser MCP; the action auto-completes.
- Wrap with the workspace ledger (`argo worker actions render`) and
  audit trail (`argo worker actions status <id>`).

This substitutes BrowserExecutor for `demo_script.md`'s HumanExecutor at
`post_listing`, and Slack-button approval for `argo worker actions approve`.

---

## 2. Pre-flight checklist

Run the verification commands in a fresh terminal **outside** any
worker context (workers must not see `ANTHROPIC_API_KEY`).

| Item | Verify with |
|---|---|
| Rust toolchain ≥ 1.75 | `rustc --version` |
| Repo on origin/main, clean | `git -C ~/code/argo status && git -C ~/code/argo rev-parse --abbrev-ref HEAD` |
| `argo` release binary built | `cargo build --release -p argo-cli && ls target/release/argo` |
| `argo` on PATH (or aliased) | `which argo` (or `alias argo=$PWD/target/release/argo`) |
| Anthropic key in **demo terminal only** | `printf '%s\n' "${ANTHROPIC_API_KEY:?unset}" \| head -c 6 && echo …` |
| ngrok or cloudflared installed | `ngrok version` or `cloudflared --version` |
| Python 3 on PATH (for browser mock) | `python3 --version` |
| `marketplace-seller-browser` vertical present | `argo vertical list \| grep marketplace-seller-browser` |
| Slack app created + signing secret captured | see §3 |

> **Sibling dependency.** The `marketplace-seller-browser` vertical
> ships from brief `mvp-demo-vertical-browser-marketplace`. If it isn't
> in `argo vertical list`, wait for that brief — or copy
> `verticals/marketplace-seller/` and flip `post_listing.executor` to
> `{ kind = "browser", binding = '…' }` per
> `crates/argo-cli/tests/browser_action_executor_smoke.rs`.

---

## 3. Slack app setup (5–10 min, one-time)

### 3.1 Create the app from a manifest

1. Visit <https://api.slack.com/apps>.
2. Click **Create New App** → **From an app manifest**.
3. Pick the workspace you'll demo against.
4. Paste the YAML in §3.2 into the manifest pane.
5. Click **Create** → **Install to workspace** → **Allow**.

### 3.2 App Manifest (paste verbatim)

```yaml
display_information:
  name: SKYCOT Demo
  description: v1 mission demo — approval gates for SKYCOT workers.
  background_color: "#1a1a1a"
features:
  bot_user:
    display_name: SKYCOT Demo
    always_online: false
oauth_config:
  scopes:
    bot:
      - chat:write
      - chat:write.public
      - incoming-webhook
settings:
  interactivity:
    is_enabled: true
    request_url: https://PASTE_PUBLIC_TUNNEL_URL_HERE/channels/slack/webhook
  org_deploy_enabled: false
  socket_mode_enabled: false
  token_rotation_enabled: false
```

> `request_url` is a placeholder — you'll edit it in §4.3 after the
> tunnel is up. Slack rejects the manifest if the URL is empty, so
> keep `https://PASTE_...` for now.

### 3.3 Capture credentials

From the app's pages, copy:

- **Basic Information → Signing Secret** → `SLACK_APPROVALS_SIGNING_SECRET`.
- **OAuth & Permissions → Bot User OAuth Token** (`xoxb-…`) →
  `SLACK_APPROVALS_BOT_TOKEN`.
- **Incoming Webhooks → Add New Webhook to Workspace** → URL goes into
  `SLACK_WEBHOOK_URL`.

Env-var names are read by `crates/argo-channels/src/slack_inbound.rs`
(inbound HMAC) and
`crates/argo-cli/src/handlers/worker_actions_notify_slack.rs` (outbound
post). Don't rename them.

---

## 3.5 Demo-mode magic-link bypass (recording only)

`ARGO_DEMO_MODE=1` mounts a single dev-only route
`POST /api/v1/demo/mint-principal` on the gateway. The route returns
a session token without an SMTP round-trip, so the operator can record
the v1.0 demo in a single take without wiring up a throwaway inbox.

**What it does** (when the gateway boots with `ARGO_DEMO_MODE=1` AND
`ARGO_GATEWAY_AUTH_BYPASS=1`):

- Mounts `POST /api/v1/demo/mint-principal`. Body: `{"email": "demo@throwaway.cc"}`.
  Response: `{"ok": true, "principal": "human:demo@throwaway.cc", "session_token": "<jwt>"}`.
- The SPA's signup form (when launched with `NEXT_PUBLIC_ARGO_DEMO_MODE=1`)
  posts to this route on the **Enter demo workspace** button instead of
  the normal "Send magic link" flow, then routes the operator straight
  to `/onboarding/welcome`.
- Every mint emits `tracing::warn!(target: "argo_cli::demo_mode", …)`
  to gateway stderr.
- Every mint writes an `auth.succeeded` audit row tagged
  `payload.demo_mode_bypass: true`, so post-recording you can confirm
  the row exists and tell demo-recording principals apart from real
  signups.

**Safety rails:**

- If `ARGO_DEMO_MODE=1` is set but `ARGO_GATEWAY_AUTH_BYPASS=1` is NOT,
  the gateway refuses to bind with:
  `ARGO_DEMO_MODE=1 cannot be used in production; set ARGO_GATEWAY_AUTH_BYPASS=1 or unset ARGO_DEMO_MODE`.
- The env var is read ONCE at gateway startup and cached. There is no
  runtime flip; a malicious request cannot enable the bypass mid-process.
- Without `ARGO_DEMO_MODE=1`, the route returns 404 (the dispatcher
  short-circuits before reaching it).

**Copy-pasteable env block** (drop into the demo terminal alongside
the other vars in §5):

```bash
# Demo-mode bypass — operator-only, recording sessions only
export ARGO_DEMO_MODE=1
export ARGO_GATEWAY_AUTH_BYPASS=1

# Also export the SPA-side mirror so the signup form switches branches
export NEXT_PUBLIC_ARGO_DEMO_MODE=1
```

**Verify it's working:**

1. `argo gateway start` — stderr should print
   `argo gateway: ARGO_DEMO_MODE=1 active — POST /api/v1/demo/mint-principal is mounted`.
2. Open `http://localhost:3000/auth/signup`. You should see a yellow
   "DEMO MODE — auth bypass active" banner at the top of the form.
3. Type any email + click **Enter demo workspace**. Browser should
   navigate to `/onboarding/welcome` without a 401.
4. Tail the gateway stderr — you should see one
   `demo-mode magic-link bypass minted principal for <email>` line per
   click.
5. Confirm the audit row: `sqlite3 ~/.argo/sessions.db "SELECT payload FROM audit_record WHERE payload LIKE '%demo_mode_bypass%' ORDER BY seq DESC LIMIT 1"`.

**Never enable this in production.** The startup guard (§3.5 above)
covers the common foot-gun, but you should also avoid setting these
env vars in any environment that isn't your own laptop during a
single recording session. Treat `ARGO_DEMO_MODE` the same way you'd
treat `RUST_BACKTRACE=1` in production — a debugging knob, not a
deployment configuration.

---

## 4. Tunnel setup (2 min)

The Slack interactivity request URL must be reachable from Slack's
servers and must terminate at the running gateway on port 7400.

### 4.1 Start the tunnel

```bash
ngrok http 7400                                    # Option A
cloudflared tunnel --url http://localhost:7400     # Option B (no signup)
```

Copy the `https://…` public URL the tool prints.

### 4.2 Build the full request URL

```
https://<public-host>/channels/slack/webhook
```

Must start with `https://` and end with `/channels/slack/webhook`
exactly (this is the `webhook_path` in `manifests/channels/slack.toml`).

### 4.3 Wire it into Slack

App config → **Interactivity & Shortcuts** → paste the URL into
**Request URL** → **Save Changes**. Slack should accept (no red error).

---

## 5. Environment variables

Set these in the **demo terminal only** (do not export globally — the
worker loop refuses to start if `ANTHROPIC_API_KEY` is set in its
environment).

```bash
# Provider key (real, paid-tier)
export ANTHROPIC_API_KEY=sk-ant-...

# Slack inbound signing — from §3.3 Basic Information
export SLACK_APPROVALS_SIGNING_SECRET=...

# Slack outbound — from §3.3 Incoming Webhooks
export SLACK_WEBHOOK_URL=https://hooks.slack.com/services/T.../B.../...
export SLACK_APPROVALS_BOT_TOKEN=xoxb-...

# Browser MCP fixture (mock; use a real CDP MCP server in production)
export ARGO_BROWSER_MCP_BIN="python3 $(pwd)/crates/argo-cli/tests/fixtures/browser_mock_mcp.py"

# Skip bearer-auth on the gateway for the demo session
export ARGO_GATEWAY_AUTH_BYPASS=1
```

Sanity check:

```bash
env | grep -E '^(ANTHROPIC_API_KEY|SLACK_|ARGO_BROWSER_MCP_BIN|ARGO_GATEWAY_AUTH_BYPASS|SKYCOT_DB_KEY_BASE64)=' | wc -l
# expect: 7
```

---

### 5.1 SQLCipher key (one extra env var)

The gateway encrypts its local SQLite database with SQLCipher
(Structured Query Language Cipher — an open-source extension to
SQLite that adds 256-bit AES encryption to every database file).
The encryption key is derived from one of two sources, in this
order of preference:

1. **`SKYCOT_DB_KEY_BASE64`** — a 32-byte secret you provide,
   encoded as base64 (a way of representing binary bytes as
   ASCII text so they can fit in an env var).
2. **macOS Keychain** (or Linux `keyring`, Windows
   `Credential Manager`) — the gateway asks the operating
   system's password vault on startup. The OS may pop a prompt
   asking "Allow access to keyring item?" — if you click No (or
   the prompt fires behind your recording overlay and you don't
   see it), the gateway refuses to start with a 503 on every
   request.

**For recording: always set `SKYCOT_DB_KEY_BASE64` explicitly.** The
keyring path is fine for daily development but the prompt is
fragile to dismiss accidentally during a screen recording. Use:

```bash
# Generate a fresh 32-byte secret + base64-encode it. Stable
# across this recording session only — drop it after recording.
export SKYCOT_DB_KEY_BASE64="$(openssl rand -base64 32)"
```

Add this to the env-var block in §5. Your sanity check from §5
becomes:

```bash
env | grep -E '^(ANTHROPIC_API_KEY|SLACK_|ARGO_BROWSER_MCP_BIN|ARGO_GATEWAY_AUTH_BYPASS|SKYCOT_DB_KEY_BASE64)=' | wc -l
# expect: 7
```

If the gateway logs `"store open failed ... derive SQLCipher key
for workspace-default tenant: ... User canceled the operation."`,
you forgot this var. Re-export it and restart `argo gateway start`.

---

## 6. Recording session (5–7 min)

Run the commands in order. Each step lists the expected output
fragment so you know to retake if something diverges.

### 6.1 Start the gateway (terminal 1)

```bash
argo gateway start --bind 127.0.0.1 --port 7400
```

Expected:

```
gateway: listening on 127.0.0.1:7400
gateway: routes mounted (channels/slack/webhook, …)
```

Leave this running. **Don't record this terminal** — keep it off-screen.

### 6.2 Init the demo workspace (terminal 2 — the one you record)

```bash
mkdir demo-seller && cd demo-seller
argo workspace init
argo vertical list | grep marketplace-seller-browser
```

Expected:

```
marketplace-seller-browser  0.1.0  Marketplace seller (browser-executed)
```

*Narration pause: ~10s — show the vertical is loaded.*

### 6.3 Drop the brief

```bash
argo work add \
  --vertical marketplace-seller-browser \
  --prompt "I have a pair of Powerslide Imperial 110mm inline skates, size 12 US, used 8x. Sell on Facebook Marketplace + eBay. Reserve \$150."
```

Expected: `queued job <job_id>` and the id printed.

*Narration: "We tell the worker the goal. Not the steps."*

### 6.4 Run the worker (≈90s)

```bash
argo work serve --vertical marketplace-seller-browser --once --max-steps 5 --json | tee /tmp/serve.json
```

Expected tail: `"stop_reason": "approval_required"`.

### 6.5 Inspect the action ledger

```bash
argo worker actions list --vertical marketplace-seller-browser --state all --json | jq -c '.[] | {state,type,requires_approval,platform}'
```

Expected: three rows — `verify_price` (autonomous), two `post_listing`
(approval-gated, one per platform).

### 6.6 Approve in Slack

Switch to Slack — the bot has posted a Block Kit message with
**Approve** / **Reject**. Click **Approve** on the eBay row.

Back in terminal 2:

```bash
argo worker actions list --vertical marketplace-seller-browser --json \
  | jq -c '.[] | select(.platform=="ebay") | {state,approved_by}'
```

Expected: `{"state":"approved","approved_by":"slack:U…"}`.

*Narration: "Inbound webhook verified Slack's v0 HMAC — signed
end-to-end, no token paste."*

### 6.7 Execute via BrowserExecutor

```bash
EBAY_ID=$(argo worker actions list --vertical marketplace-seller-browser --json \
  | jq -r '.[] | select(.platform=="ebay") | .action_id')
argo worker actions execute "$EBAY_ID" --json
```

Expected: `"state": "executing"` then `"state": "completed"` plus an
`execution_bundle_path` under `.skycot/outputs/<job>/actions/<id>/`.

### 6.8 Render the ledger + show audit trail

```bash
argo worker actions render --vertical marketplace-seller-browser
cat .skycot/approval_queue.md       # facebook_marketplace row
cat .skycot/completed_actions.md    # ebay row
argo worker actions status "$EBAY_ID"
```

Expected status transitions:
`pending → approved → executing → completed`, `approved_by: slack:U…`.

*Closing narration: "Every transition is a memory row. The audit
trail is the data model."* **Stop recording here.**

---

## 7. Post-recording

### 7.1 Save the artifacts

```
docs/skycot/demos/v1-mission-mvp.mp4         # the screen capture
docs/skycot/demos/v1-mission-mvp.md          # transcript (skeleton below)
```

Transcript skeleton:

```markdown
# v1 mission demo — transcript

Recorded: <YYYY-MM-DD>   Duration: <MM:SS>   Argo commit: <sha>

## Setup
- Vertical: marketplace-seller-browser
- Channel: Slack approval + BrowserExecutor (post_listing)
- Gateway: 127.0.0.1:7400 via <tunnel host>

## Beats
- 0:00 brief dropped · 0:15 worker runs → `approval_required`
- 1:40 Slack notify · 1:55 operator clicks Approve
- 2:05 execute → BrowserExecutor → `completed`
- 2:30 render ledger · 2:55 audit trail · 3:10 stop
```

### 7.2 Commit message template

```
docs: ship v1 mission demo recording + transcript

Adds the recorded v1 mission demo (5-7 min) and accompanying
transcript. Stack: marketplace-seller-browser vertical, Slack-button
approval via /channels/slack/webhook, BrowserExecutor against the
browser_mock_mcp.py fixture.

Refs: mvp-demo-runbook, mission-v1-demo-recording.
```

### 7.3 PR checklist for the v1 demo sign-off

- [ ] `docs/skycot/demos/v1-mission-mvp.mp4` committed (LFS if > 50 MB).
- [ ] `docs/skycot/demos/v1-mission-mvp.md` transcript matches the
      recording timestamps.
- [ ] Recorded commit sha cited in the transcript.
- [ ] BrowserExecutor branch in the demo (not HumanExecutor).
- [ ] Slack-button approval visible on screen (not CLI approve).
- [ ] `BUILD_STATUS.md` mission row flipped to IN_REVIEW with PR #.

---

## Cross-references

- Demo script being narrated: [`../demo_script.md`](../demo_script.md).
- BrowserExecutor env-var + fixture pattern:
  `crates/argo-cli/tests/browser_action_executor_smoke.rs`.
- Slack inbound HMAC: `crates/argo-channels/src/slack_inbound.rs`.
- Slack manifest (env-vars, `webhook_path`): `manifests/channels/slack.toml`.
- Gateway bind + auth-bypass: [`security.md`](security.md),
  [`production-deploy.md`](production-deploy.md).
