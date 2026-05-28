# pitch-agent-rehearsal-ui-and-gateway

**Priority**: P0 — v1.0 demo MP4 critical path (Pitch Agent operator-facing surface)
**Source**: 2026-05-29T01:00Z (Melbourne 11:00 AEST) — operator-authorized overnight implementation
**Authored against**: origin/main `a5df90c41`
**Authored by**: claude conductor (interactive session, operator-asleep, full overnight authority)

## Why this brief exists

The Pitch Agent has full backend plumbing — P-01 truth-boundary capability schema, P-02 runtime enforcement, P-03 voice-io crate, P-04 canonical-Q&A skill, P-05 worker template (`packs/pitch-agent/`), P-08 eval suite passing **111/111**. The `Pitch` worker exists in `/api/v1/team`. The `argo-skill::canonical_qa::CanonicalQa` matcher is a deterministic in-process IDF-weighted lexical lookup — no LLM call required.

**What's missing**: an operator-facing surface to actually USE the Pitch Agent during the demo MP4. There is no `/pitch` route in the SPA, no `/api/v1/pitch/ask` gateway endpoint, no chat UI. The pitch-eval CLI demonstrates the platform works but is not demo-able on camera.

This brief closes that gap with the minimum-viable demo surface: text-only canonical-Q&A chat at `/pitch`, backed by a new gateway endpoint that wraps existing `argo-skill::canonical_qa` + `argo-capability::evaluate_claim`.

## Scope (TIGHT — overnight ship)

### 1. Gateway endpoint: `POST /api/v1/pitch/ask`

New file: `crates/argo-cli/src/handlers/gateway/routes_pitch_ask.rs`

Wire in: `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`

Request body (JSON):
```json
{
  "question": "What's the tech stack?",
  "channel": "Rehearsal" | "WebEmbed" | "LiveMeeting" | "Chat",
  "tenant_id": "<optional override; defaults to workspace tenant>"
}
```

Response (JSON, 200 OK):
```json
{
  "matched": true,
  "entry_id": "tech-stack",
  "answer": "Rust single-binary core with a JSON-RPC plugin ABI...",
  "declared_claim_classes": ["architecture"],
  "disposition": "Allow" | "Escalate" | "Block",
  "block_reason": null,
  "channel": "Rehearsal",
  "latency_ms": 1
}
```

When no canonical entry matches above the similarity_threshold (0.45 per the manifest):
```json
{
  "matched": false,
  "answer": "I'll get back to you on that one — let me check with the founder.",
  "disposition": "Escalate",
  "channel": "Rehearsal",
  "latency_ms": 1
}
```

When matched but claim disposition is `Block` per truth-boundary policy:
```json
{
  "matched": true,
  "entry_id": "valuation-question",
  "answer": "<the canonical refusal text>",
  "disposition": "Block",
  "block_reason": "forbidden_class: valuation",
  "channel": "WebEmbed",
  "latency_ms": 1
}
```

Implementation outline (~120 LOC Rust):
```rust
// load matcher from skill-library/canonical-qa/corpora/skycot/
let matcher = CanonicalQa::load_from_dir(&corpus_dir)?;
let hit = matcher.match_question(&req.question);
match hit {
    Some(entry) => {
        let policy = load_policy_for_channel(req.channel)?;
        let claims: Vec<ClaimCandidate> = entry.declared_claim_classes.iter()
            .map(|c| ClaimCandidate { class: c.clone(), text: entry.answer.clone() })
            .collect();
        let disposition = claims.iter().map(|c| evaluate_claim(c, &policy, req.channel)).max();
        emit_audit_event("pitch.canonical_qa_matched", &entry.id, disposition);
        // ... return response
    }
    None => {
        emit_audit_event("pitch.canonical_qa_miss", question_hash, "Escalate");
        // ... return graceful fallback
    }
}
```

Must:
- Audit-emit every Q&A as `pitch.canonical_qa_matched` or `pitch.canonical_qa_miss` with action_kind variant
- Bind tenant_id as 16-byte BLOB (consistent with #2631)
- CORS headers via `cors::cors_headers_for_request(req)` (consistent with the genesis SSE pattern)
- LOC budget: keep file under 200 LOC

### 2. SPA chat page: `/pitch`

New file: `web/app/(app)/pitch/page.tsx`

Layout: brand-loud hero ("Onboard a new teammate." pattern) + chat input + scrolling Q&A history.

State (React useState):
- `history: Array<{ role: 'user' | 'agent', text: string, meta?: ResponseMeta }>`
- `pending: boolean`
- `question: string`

On submit:
- Append user message to history
- POST `/api/v1/pitch/ask` with `{ question, channel: "Rehearsal" }`
- Append agent response to history with metadata pill (entry_id + disposition + claim class)
- Clear input

Hero copy (per `pitch-agent-v1-storyboard.html` Scene 1):
> "Watch what happens when I try to pitch Skycot."

Sub: "Rehearse with the Pitch Agent. Truth-bounded, evidence-led, brand-safe."

Input placeholder: "Ask anything an investor might ask. Try: What's the tech stack? How much does it cost?"

Visual: brand-loud purple→pink gradient hero, single-column 640px max-width, Audiowide on hero, Inter on chat bubbles.

LOC budget: ~250 LOC TypeScript.

### 3. Sidebar nav entry

`web/components/skycot/nav-items.tsx`: add `Pitch` entry between `Genesis` and `Cohorts` (or wherever feels right — operator can rearrange).

Icon: `Mic` from `lucide-react` (or a similar pitch-shaped icon).

### 4. API client

New file: `web/lib/api/pitch.ts` exporting `askPitch(question, channel)` that calls the new endpoint, with TypeScript types matching the gateway response shape.

### 5. Tests

- `crates/argo-cli/tests/pitch_ask_endpoint.rs` integration test:
  - canonical-hit returns Allow with matched entry_id
  - canonical-miss returns Escalate with fallback text
  - claim-blocked returns Block with reason
  - audit row emitted for each
  - cross-tenant negative: T1's call doesn't see T2's corpus (if tenant scoping is in scope)
- `web/lib/api/pitch.test.ts` — typed-client wire test (matches existing `genesis.test.ts` pattern)
- `web/app/(app)/pitch/page.test.tsx` — vitest covering: empty state, submit happy-path, error banner on 5xx

## Acceptance

- [ ] `POST /api/v1/pitch/ask` returns canonical answer for "What's the tech stack?" within 50ms
- [ ] `POST /api/v1/pitch/ask` returns Block for valuation-class question on WebEmbed channel
- [ ] `POST /api/v1/pitch/ask` returns Escalate fallback for unrecognized question
- [ ] `/pitch` page in SPA renders chat UI, accepts text input, displays response with metadata
- [ ] One canonical hit + one miss + one block scenario each emit a distinct audit row visible in `/audit` within 5s
- [ ] `cargo clippy -p argo-cli -- -D warnings` passes
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` passes
- [ ] `pnpm typecheck` + `pnpm lint` in web/ passes
- [ ] `cargo test -p argo-cli --test pitch_ask_endpoint` 4/4 pass
- [ ] `cargo test -p argo-skill canonical_qa` regression untouched (pre-existing tests still pass)
- [ ] vitest `web/lib/api/pitch.test.ts` + `web/app/(app)/pitch/page.test.tsx` green
- [ ] `pitch-eval` regression untouched (111/111 still passes)

## Out of scope (DEFER to follow-ups)

- Voice I/O for the chat (P-03 voice-io crate is built but not wired here — text-only for v1.0)
- Web-embed channel (P-06 DROPPED)
- Zoom/Live-meeting channel (P-07 BLOCKED on macOS audio routing)
- LLM-driven free-form responses (out of scope; canonical-Q&A only for v1.0)
- Lead capture flow (deferred)
- Multi-turn conversation memory (single-shot Q&A only)

## May-touch

- **NEW Rust**: `crates/argo-cli/src/handlers/gateway/routes_pitch_ask.rs`, `crates/argo-cli/tests/pitch_ask_endpoint.rs`
- **EDIT Rust**: `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` (1-line wire-up), `crates/argo-cli/src/handlers/gateway/mod.rs` (declare new module if needed)
- **EDIT Rust audit**: `crates/argo-audit/src/event_kind.rs` (add 2 new variants: `PitchCanonicalQaMatched`, `PitchCanonicalQaMiss`) — keep under split-budget per #2295's audit headroom rules
- **NEW SPA**: `web/app/(app)/pitch/page.tsx`, `web/app/(app)/pitch/page.test.tsx`, `web/lib/api/pitch.ts`, `web/lib/api/pitch.test.ts`
- **EDIT SPA**: `web/components/skycot/nav-items.tsx` (1 new entry)
- **EDIT docs**: `docs/skycot/demos/v1-mission-mvp-runbook.md` add a "Pitch Agent demo path" subsection with the new `/pitch` route

## Notes for the worker

- The pitch-eval suite at `crates/argo-eval/src/pitch_runner.rs` shows EXACTLY how to wire canonical_qa + truth-boundary together. **Mirror that pattern**, don't reinvent it. The pitch_runner code is the proven path.
- The `argo-skill::canonical_qa::CanonicalQa::load_from_dir(path)` API is the entry point. Each entry has `id`, `prompts: Vec<String>`, `answer: String`, `declared_claim_classes: Vec<String>`.
- For audit emission, look at how `routes_genesis_stream.rs` emits the `vertical_ready` audit row at the end — same pattern with `emit_audit()`.
- Operator authorization: `--skip-clippy` lane authorized for this PR (P0 demo-headline, worker-side full verification + 111/111 pitch-eval regression untouched).
- Reuse existing canonical_qa corpus at `skill-library/canonical-qa/corpora/skycot/` — DO NOT author new content. The 5 corpus files are operator-reviewed and load-bearing for the pitch-eval suite.

## Why this is THE Pitch Agent demo unlock

- The platform invariants from CLAUDE.md hold: vertical behavior in manifests (pack.toml, corpora, policies), not runtime forks
- Truth-boundary enforcement (P-02 #2295) fires on every answer
- Audit chain (P-08 #2401) captures every Q&A
- No LLM cost or latency — deterministic lexical match
- Same code path the eval suite proves works 111/111

The demo MP4 can now record: Mark types "What's the tech stack?" → instant on-brand answer; types "What's the valuation?" → graceful refusal with capability-policy citation visible in `/audit`. That IS Scene 1 of the storyboard, made literal.
