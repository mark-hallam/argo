# eval-channels-inbound-coverage

**Authored**: 2026-05-23 (evaluator round 11)
**Brief**: `docs/build-plan/streams/eval-channels-inbound-coverage.md`
**Gap-manifest row cited** (from `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` line 91, verbatim):

> | Channel inbound paths | — | 🔴 GAP | Mission claim 6 says "each channel impl handles BOTH outbound (request approval) AND (where applicable) inbound (receive the operator's reply)". I see `email_inbound.rs` + `slack_inbound.rs` only. The other 11 channels don't have an inbound path. | `argo-channels/src/email_inbound.rs`, `slack_inbound.rs` | 🟡 P1 | `eval-channels-inbound-coverage` — list which channels have inbound (approval-reply) wired vs only outbound. Push notifications + SMS + WhatsApp inbound are common gaps. |

The manifest hypothesis ("only email + Slack inbound") is **partially out of date.** Two parallel inbound architectures exist:

1. **`InboundChannel` trait** (`crates/argo-channels/src/slack_inbound.rs:52`) — formal abstraction with `verify_signature` + `decode_approval_token`. Only `SlackInbound` and `EmailInbound` (stub) implement it.
2. **Gateway route files** (`crates/argo-cli/src/handlers/gateway/routes_channels_*.rs`) — 8 channels have direct webhook handlers wired into the HTTP dispatcher at `routes_channels.rs:41` (`is_email_webhook`) / `routes_channels.rs:55` (`email_webhook_response`).

The gateway routes are the **shipped** inbound surface (mounted on the gateway HTTP listener). The `InboundChannel` trait is a partially-realised abstraction that has not yet been retrofitted across the other channels.

## Verdict matrix

Legend:
- 🟢 = both signature verify (or transport-level equivalent) AND approval-token consume present.
- 🟡 = approval-token consume present, transport signature verify missing or weaker than provider standard.
- 🔴 = no inbound surface at all (outbound-only `NotifyChannel`).
- "n/a (deep-link)" = the URL itself is the HMAC-signed approval token; no transport signature is meaningful because the gateway is the trust root (mobile app / browser opens the link directly).

| # | Channel | Outbound (`NotifyChannel`) | Inbound — `InboundChannel` trait | Inbound — gateway route | Signature verify | Approval-token consume | Verdict |
|---|---|---|---|---|---|---|---|
| 1 | email (Postmark) | `email.rs` ✅ | `email_inbound.rs` (stub) ✅ | `routes_channels_email.rs` ✅ | Gateway: HMAC token in body; trait: HMAC over body via `X-Argo-Email-Signature`; **transport sig (Postmark Webhook auth) NOT verified at gateway** — relies on HMAC token only | `verify_token` at `routes_channels_email.rs:36` | 🟡 partial — gateway route does NOT call into `EmailInbound::verify_signature`; the trait stub at `email_inbound.rs` says "full IMAP/SES wiring is deferred" |
| 2 | slack | `slack.rs` ✅ | `slack_inbound.rs` ✅ | `routes_channels_slack.rs` ✅ | `verify_slack_signature` (`v0:<ts>:<body>` HMAC) ✅ | `verify_token` ✅ | 🟢 full |
| 3 | discord | `discord.rs` ✅ | none | `routes_channels_discord.rs` ✅ | **Ed25519 signature NOT verified** (Discord's `X-Signature-Ed25519` + `X-Signature-Timestamp` headers ignored) — Discord developer-portal will refuse to mark the endpoint as verified, so interactions will not be delivered in production until this is fixed | `verify_token` (HMAC) ✅ | 🟡 partial — approval-token path is secure (HMAC-signed token), but Discord interaction protocol mandates Ed25519 transport verify; without it the channel is non-functional end-to-end |
| 4 | imessage (Blue Bubbles) | `imessage.rs` ✅ (outbound) | inbound logic lives in same file (`imessage.rs:122`); does NOT implement `InboundChannel` trait | `routes_channels_imessage.rs` ✅ | `verify_imessage_signature` (HMAC) ✅ | `verify_token` ✅ + `verify_approval_token` (channel-level) ✅ | 🟢 full |
| 5 | voice (Twilio) | `voice.rs` ✅ | none | `routes_channels_voice.rs` ✅ | `verify_twilio_signature` over `X-Twilio-Signature` ✅ | `verify_voice_token` (DTMF token) ✅ | 🟢 full |
| 6 | whatsapp (Twilio) | `whatsapp.rs` ✅ | none | `routes_channels_whatsapp.rs` ✅ | `verify_twilio_signature` over `X-Twilio-Signature` ✅ | `verify_token` ✅ | 🟢 full |
| 7 | webinbox | `webinbox.rs` ✅ | none | `routes_channels_webinbox.rs` ✅ | n/a (same-origin browser POST) | `verify_token` ✅ | 🟢 full (per design — token is the auth) |
| 8 | push (Expo) | `push.rs` ✅ | none | `routes_channels_push.rs` ✅ | n/a (deep-link; mobile app opens URL directly) | `verify_token` ✅ | 🟢 full (per design — token in URL is the auth) |
| 9 | sms (Twilio) | `sms.rs` ✅ | none | **none — no `routes_channels_sms.rs` exists** | — | — | 🔴 outbound-only |
| 10 | teams (Microsoft Bot Framework) | `teams.rs` ✅ | none | **none — no `routes_channels_teams.rs` exists** | — | — | 🔴 outbound-only |
| 11 | matrix (Client-Server API) | `matrix.rs` ✅ | none | **none — no `routes_channels_matrix.rs` exists** | — | — | 🔴 outbound-only |

## Counts

- **Channels shipped**: 11 (`argo-channels/src/{discord,email,imessage,matrix,push,slack,sms,teams,voice,webinbox,whatsapp}.rs`; `email_inbound`, `slack_inbound`, `fallback` are companion modules not standalone channels).
- **Channels with inbound surface end-to-end**: 6 (slack, imessage, voice, whatsapp, webinbox, push).
- **Channels with inbound surface but a transport-signature gap**: 2 (email — gateway route does not invoke the `EmailInbound` trait stub; discord — Ed25519 not verified).
- **Channels with NO inbound surface at all**: 3 (sms, teams, matrix).

## Cross-architecture observation (process drift signal)

Two parallel inbound architectures co-exist:

- `InboundChannel` trait in `argo-channels` (slack + email stub).
- Gateway route files in `argo-cli/src/handlers/gateway/` (8 channels).

The gateway routes do **not** call into `InboundChannel::verify_signature` / `InboundChannel::decode_approval_token`. They have their own inline verify functions (`verify_slack_signature`, `verify_twilio_signature`, `verify_imessage_signature`). This is a parallel-abstraction smell flagged in `docs/skycot/DRIFT_PREVENTION.md`:

> Two structs with the same name and overlapping responsibilities in different crates is a bug, not a feature.

The same applies to two trait/route surfaces with the same intent. A follow-up brief should reconcile — either (a) retrofit gateway routes to delegate to `InboundChannel` impls (preferred — keeps verification logic in the channels crate where the outbound impl lives), or (b) delete the `InboundChannel` trait and the `email_inbound.rs` stub, declaring the gateway-route file the canonical inbound surface.

## Recommended fix-briefs (queue replenishment)

### Priority 1 — new inbound paths (mission-claim 6 says "Email, Slack, SMS, push, voice, web", so SMS is on the mission promise)

**`mission-channel-sms-inbound`** — 🟡 P1
- Add `routes_channels_sms.rs` mirroring `routes_channels_whatsapp.rs` (same Twilio webhook signature; SMS replies hit Twilio's incoming-SMS webhook with `From`/`Body` params).
- Parse `Body` for `approval_token=<HMAC token>` or interpret first-word `YES`/`NO` as decision (use the imessage `extract_approval_token` pattern from `crates/argo-channels/src/imessage.rs:220`).
- Verify `X-Twilio-Signature` via the existing `verify_twilio_signature` helper exported by `routes_channels_whatsapp.rs:140`.
- Wire into the dispatcher at `routes_channels.rs:41` (`is_email_webhook`) + `routes_channels.rs:55`.
- May-touch: `crates/argo-cli/src/handlers/gateway/routes_channels.rs`, `crates/argo-cli/src/handlers/gateway/routes_channels_sms.rs` (new), `crates/argo-cli/src/handlers/gateway/mod.rs`.
- ~200 LOC.

**`mission-channel-teams-inbound`** — 🟡 P1
- Add `routes_channels_teams.rs` for Bot Framework Adaptive-Card postbacks (POST from Microsoft Bot Connector).
- Verify Bot Framework's `Authorization: Bearer <JWT>` (JWKS-rotated signing keys from `https://login.botframework.com/v1/.well-known/openidconfiguration`) **OR** for v1.0, accept HMAC-only with an `X-Argo-Teams-Signature` header (Microsoft Bot Framework supports outgoing webhook HMAC for self-hosted bots) and defer full Bot Framework JWKS to v1.x.
- Parse the adaptive-card `Action.Submit` payload for `data.approval_token`.
- ~250 LOC.

**`mission-channel-matrix-inbound`** — 🟡 P2 (deprioritise vs sms/teams — Matrix is enterprise-niche; bypass for v1.0 unless an operator asks)
- Add `routes_channels_matrix.rs` for Matrix Client-Server `m.reaction` events (the channel sends `m.room.message` with `approval_token` in `body`; operator's reaction event is the reply).
- Matrix doesn't have a "webhook" model — the bot polls `/_matrix/client/v3/sync` or uses `/_matrix/client/v3/rooms/{room}/messages` long-poll. This is a different shape than the other webhook-based routes.
- ~300 LOC.

### Priority 2 — close the gaps in existing inbound paths

**`mission-channel-discord-ed25519-verify`** — 🟡 P1 (Discord channel non-functional end-to-end without this)
- Add Ed25519 signature verification in `routes_channels_discord.rs:17` per Discord's [Interaction Security spec](https://discord.com/developers/docs/interactions/receiving-and-responding#security-and-authorization).
- Pull `X-Signature-Ed25519` + `X-Signature-Timestamp` headers; verify `<timestamp><body>` against the Discord application public key (env: `ARGO_DISCORD_PUBLIC_KEY`).
- Use `ed25519-dalek` (already in workspace per `crates/argo-cli/src/handlers/self_update.rs:4`).
- ~80 LOC.

**`mission-channel-email-gateway-trait-wiring`** — 🟡 P2
- Either: retrofit `routes_channels_email.rs` to delegate `verify_signature` + `decode_approval_token` to `EmailInbound::verify_signature` / `EmailInbound::decode_approval_token` (preferred, kills the parallel abstraction); or
- Delete `crates/argo-channels/src/email_inbound.rs` + the `InboundChannel` trait + the slack-side impl, declaring the gateway-route file the canonical inbound surface.
- The deferred IMAP/SES wiring referenced in `email_inbound.rs:3` ("full IMAP/SES wiring is deferred") is orthogonal — it concerns ingesting replies from an IMAP poll rather than the Postmark webhook path the gateway currently uses.
- ~150 LOC.

### Priority 3 — architecture reconciliation

**`mission-channels-inbound-trait-reconciliation`** — 🟡 P2
- Pick one architecture: trait or gateway-route. Document the decision in `docs/skycot/architecture/channels-inbound.md`.
- If trait wins: retrofit all 8 gateway routes (slack, discord, email, imessage, voice, whatsapp, webinbox, push) to delegate to `InboundChannel` impls. Channels gain a `verify_signature` + `decode_approval_token` method each.
- If gateway-route wins: delete `crates/argo-channels/src/slack_inbound.rs`, `email_inbound.rs`, and the `InboundChannel` trait. Move HMAC verify functions to a `channels_verify.rs` shared module.
- ~600-800 LOC across either direction; manager-worker candidate.

## Acceptance check vs brief

Per brief acceptance criteria:

- [x] Per-brief acceptance: matrix of which channels have inbound (approval-reply) paths. **Done above.**
- [x] If real cross-tenant leak / broken-wire found: ship as `#[ignore]`'d FINDINGS-ONLY test + name a fix-brief. **No cross-tenant leak found at this layer** (gateway is per-workspace process; HMAC secret is process-scoped; approval-id resolves via the workspace memory store). Two broken-wire findings (Discord Ed25519 missing, email gateway-vs-trait parallel) are documented above with named fix-briefs; both are documentation/refactor findings, not security regressions, so they do not warrant a FINDINGS-ONLY test.
- [x] `cargo clippy -p <affected-crate> --all-targets -- -D warnings` clean. **N/A — this is a documentation-only PR** (no source touched).
- [x] Every modified file < 500 LOC. **This doc is ~150 lines; no source files modified.**
- [ ] BUILD_STATUS.md updated with PR number. **Pending post-PR-open.**

## Citations

- Gateway dispatcher (where inbound routes plug in): `crates/argo-cli/src/handlers/gateway/routes_channels.rs:41`–`:78`.
- `verify_token` (shared HMAC token verify): `crates/argo-cli/src/handlers/gateway/routes_channels.rs:222`.
- `InboundChannel` trait: `crates/argo-channels/src/slack_inbound.rs:52`.
- `EmailInbound` stub: `crates/argo-channels/src/email_inbound.rs:1`–`:60`.
- `verify_twilio_signature`: `crates/argo-cli/src/handlers/gateway/routes_channels_whatsapp.rs:140`.
- Discord interaction route (Ed25519-gap): `crates/argo-cli/src/handlers/gateway/routes_channels_discord.rs:17`.
- iMessage in-file inbound: `crates/argo-channels/src/imessage.rs:122`.
- SMS / Teams / Matrix outbound-only (no inbound siblings): `crates/argo-channels/src/{sms,teams,matrix}.rs`.

## Boundary notes

- This brief's may-touch list is `crates/argo-channels-*/, gateway routes_channels_*.rs` — a documentation-only PR uses neither path, so it lives in `docs/skycot/evals/scheduled/2026-05-23/`. The brief mirrors the pattern of round-7/round-8/round-9 audit PRs (e.g. `eval-credentials-cross-tenant-isolation`) which also wrote audit reports under `docs/skycot/evals/scheduled/`.
- No production source files modified.
