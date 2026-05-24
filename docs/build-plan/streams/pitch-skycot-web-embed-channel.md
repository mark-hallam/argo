# pitch-skycot-web-embed-channel

**Owner**: claude.
**Authored against**: origin/main `73ac6e06c` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh pitch-skycot-web-embed-channel`.
**Blocked by**: `pitch-truth-boundary-runtime-enforcement` (P-02) MERGED + `pitch-voice-io-provider-agnostic-crate` (P-03) MERGED + `pitch-agent-worker-template` (P-05) MERGED.

🔴 **P0 v1.0 ship-blocker · Track P (Pitch Agent) · brief P-06 of 8**

The web-embed channel — a public route `/pitch` on skycot.com and an embeddable widget that any tenant can drop into their site. Channel 2 of the storyboard. Registers as `ChannelKind::WebEmbed` (per P-02) so the truth-boundary policy enforces the public-channel forbidden-claim list.

## Why this brief exists

The v1.0 demo storyboard's Scene 3 requires the Pitch Agent to be reachable through a public web surface — anonymous visitors ask questions live, the agent responds with voice + text, lead-capture is a first-class skill, the truth boundary holds under public-channel rules (stricter than private rehearsal).

This brief is also the second-channel proof for claim 6 (multi-channel): same worker, different surface, different policy.

## Source of truth

- **Storyboard**: [`docs/skycot/demos/pitch-agent-v1-storyboard.html`](../../skycot/demos/pitch-agent-v1-storyboard.html) — Scene 3 (the visitor on skycot.com/pitch, the customer-count refusal in 3b)
- **Memory**: `project_pitch_agent_v1_demo.md`; existing landing/marketing pages per `project_brand_consistency_pass`
- **Brand register**: D3 brand-consistency pass — Audiowide hero, purple→pink gradient, warm radial-blob bg, #1e1b4b ink
- **Voice loop**: P-03 `argo-voice-io` (consumed via a thin server-side broker)
- **Worker template**: P-05 — provides the persona, skills bundle, and the `tenant-public-pitch` policy ID

## Scope

### 1. Public route at `web/app/(public)/pitch/page.tsx`

A standalone landing page for the agent (not embedded). Brand-consistent (D3 pass).

Layout:

- Audiowide hero: "Ask Pitch anything about Skycot."
- Centred chat panel: input + transcript area + voice-toggle
- Subtle pill: "Truth-boundary enforced — every answer sourced" linking to an explainer modal
- Footer: "What I can't tell you" — a transparent list of forbidden claim classes for the public channel (e.g. valuation, customer count, unannounced roadmap). This is a *trust signal* — being transparent about what the agent will refuse builds credibility.

The page is the primary surface; the widget (next section) is the same UI in a smaller frame for embedding.

### 2. Embeddable widget

`web/app/(public)/pitch/widget/page.tsx` — minimal-chrome frame for embedding via `<iframe>`. Plus a one-line embed snippet:

```html
<script src="https://skycot.com/pitch/embed.js" data-tenant="skycot"></script>
```

`embed.js` injects a floating-corner widget that opens to an `<iframe>` of `/pitch/widget?tenant=<tenant>`.

Multi-tenant inference: the embed.js takes `data-tenant`. The iframe URL carries it. The backend resolves the right worker template instance per tenant. P-05's tenant-scoped policy resolution does the rest.

### 3. WebSocket session API

The page opens a session against a new gateway endpoint:

```
POST /api/v1/pitch/sessions
  { tenant_id, audience_tag: "investor" | "buyer" | "press" | "anonymous" }
  -> { session_id, ws_url, capabilities }

WS  /api/v1/pitch/sessions/<session_id>
  Client -> server frames:
    { type: "user_text", text: "..." }
    { type: "user_audio_chunk", pcm: "base64..." }
    { type: "interrupt" }
  Server -> client frames:
    { type: "agent_text_chunk", text: "..." }
    { type: "agent_audio_chunk", pcm: "base64...", utterance_id: "..." }
    { type: "transcription", text: "...", is_final: bool }
    { type: "claim_blocked", refusal: "...", class: "valuation" }
    { type: "lead_capture_offered", offer: "data-room | calendar | pricing-pdf" }
    { type: "approval_pending", reason: "..." }
```

The gateway proxies the WS through to the agent runner, which holds the `VoiceSession` (from P-03). Channel adapter on the runner side registers as `ChannelKind::WebEmbed`.

### 4. Lead-capture flow

When the agent offers a lead-capture path (Scene 3a "drop your email and I'll send it"), the client UI surfaces an inline email input with a "Send me the link" button. The skill that produces the offer is shared (`lead-capture-broker`) — register it in the worker template bundle if not already present.

The captured lead writes to:
- The tenant's CRM via existing MCP connector (Hubspot / Close / Salesforce — tenant chooses in their pack)
- A `lead_capture_events` table (audit + analytics)

Both writes are tenant-scoped per the multi-tenant invariant.

### 5. Per-channel policy enforcement

The runtime gate from P-02 will see `ChannelKind::WebEmbed` and look up the channel-override policy from the worker template (P-05). The example Skycot policies (`policies/truth-boundary/skycot-public-pitch.toml` from P-01) define:

- Revenue, valuation: `Forbid` on all channels
- Customer count: `Forbid` on `WebEmbed` + `LiveMeeting`, `Allow` on `Rehearsal`
- Unannounced roadmap: `Escalate` on all channels (the agent asks the operator)

This brief does NOT redefine these — they live in P-01's policies. This brief just registers the channel.

### 6. Channel adapter

Add to `crates/argo-channels/`:

```rust
pub struct WebEmbedChannel {
    tenant_id: TenantId,
    session_id: SessionId,
    audience_tag: String,
    voice: Option<VoiceSession>,
}

impl ChannelAdapter for WebEmbedChannel {
    fn kind(&self) -> ChannelKind { ChannelKind::WebEmbed }
    // ... existing trait methods
}
```

Brand-restraint note: the public widget UI is brand-medium register (per the `project_brand_consistency_pass` memory's D3 pass). The agent's name pill, the gradient corners, the warm bg — yes. The aggressive Audiowide hero on the widget itself — only at default size; smaller frames downshift to Inter.

### 7. Tests

- E2E: visitor types a question → agent responds with text + voice chunks (mock voice provider via P-03 stub)
- E2E: visitor asks "What's your valuation?" → response is the refusal variant + audit row shows `ClaimBlocked` + UI shows the soft refusal indicator
- E2E: visitor asks "Pricing?" → agent offers lead-capture path → email input renders → submission persists to CRM mock
- Cross-tenant: two iframes with different `data-tenant` resolve to different worker instances; cross-talk negative test
- Audience tag: `?audience=investor` vs `?audience=buyer` pulls different canonical-qa answers (per P-04)

## Files you may touch

- `web/app/(public)/pitch/page.tsx` (new)
- `web/app/(public)/pitch/widget/page.tsx` (new)
- `web/public/pitch/embed.js` (new — small loader, ~80 LOC)
- `web/components/pitch/*.tsx` (new — chat panel, voice toggle, lead-capture inline form)
- `crates/argo-cli/src/handlers/gateway/routes_pitch.rs` (new — WS endpoint + session create)
- `crates/argo-channels/src/web_embed.rs` (new)
- `crates/argo-channels/src/lib.rs` (re-export)
- `crates/argo-channels/tests/web_embed_e2e.rs` (new)

## Files you must NOT touch

- `crates/argo-voice-io/` — P-03 owns it; consume only
- `crates/argo-capability/truth_boundary*` — P-01 owns; the channel doesn't add policy logic
- The marketing landing page or other public routes — `/pitch` is its own route under `(public)`, don't refactor neighbours
- Audit pipeline beyond consuming existing audit events

## Acceptance criteria

- [ ] `/pitch` route renders with the D3 brand pass
- [ ] WS session opens, exchanges frames, closes cleanly
- [ ] Voice path works with the stub provider (real provider per the manifest, swappable)
- [ ] Truth-boundary refusals render distinctly (soft pill or icon, not error styling)
- [ ] Lead-capture inline form submits to CRM mock + audit row persists
- [ ] `data-tenant` correctly resolves to per-tenant worker + policy
- [ ] Cross-tenant negative test passes
- [ ] Mobile responsive
- [ ] No file >500 LOC

## Verification

```bash
cargo build -p argo-channels -p argo-cli
cargo test -p argo-channels web_embed
pnpm --dir web build
pnpm --dir web dev
# Visit http://localhost:3000/pitch
bash scripts/check-brief-freshness.sh pitch-skycot-web-embed-channel
```

Manual:
```bash
open http://localhost:3000/pitch
# Ask "How do you compare to Lindy?" — confirm hot-path canonical-qa response <300ms
# Ask "What's your valuation?" — confirm refusal renders with soft pill
# Ask "Pricing?" — confirm lead-capture inline form
```

## Cross-references

- **Blocked by**: P-02 (runtime gate), P-03 (voice-io), P-05 (worker template)
- **Sibling**: `pitch-meeting-orchestrator-zoom-channel` (P-07) — the live-meeting channel using the same pattern
- **Memory**: `project_pitch_agent_v1_demo.md`, `project_brand_consistency_pass.md`, `project_settings_page_design_locked.md` (for command-palette consistency)
- **Storyboard**: Scene 3a + 3b are the user-facing proof
