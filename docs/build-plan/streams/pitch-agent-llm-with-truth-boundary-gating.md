# pitch-agent-llm-with-truth-boundary-gating

**Priority**: P0 — v1.0 demo Pitch Agent depth + investor-flexibility unlock
**Source**: 2026-05-29T22:30Z operator-directed extension of the current canonical-QA-only path
**Authored against**: origin/main `1e075af00`
**Authored by**: claude conductor (interactive session, operator overnight authority)

## Why this brief exists

The current `/api/v1/pitch/ask` endpoint (shipped via #2640) is pure deterministic canonical-QA lookup. That's great for the top-N investor questions but it returns "I'll get back to you on that one — let me check with the founder" for ANY question outside the operator-reviewed corpus.

For the v1.0 demo MP4, operators (and live investors) will ask questions the corpus doesn't cover. A realistic Pitch Agent demo needs:

1. **Canonical-QA fast path** for the common investor questions (already shipped — 0ms, no LLM cost)
2. **LLM-backed fallback** for novel questions, with the truth-boundary policy as system-prompt context
3. **Post-LLM truth-boundary check** — scan the LLM's response for forbidden claim classes; if detected, swap in the canonical refusal text instead of the LLM's draft
4. **Company-specific shaping** — company facts loaded into LLM context so it answers consistently with operator-reviewed facts

This brief implements the hybrid path. It does NOT replace canonical-QA; it adds the LLM-fallback layer beneath it.

## Scope

### 1. Extend `/api/v1/pitch/ask` request/response shape

Request body (additive, backwards-compatible):
```json
{
  "question": "How does your pricing scale on a 50-person team?",
  "channel": "Rehearsal" | "WebEmbed" | "LiveMeeting" | "Chat",
  "tenant_id": "<optional override>"
}
```

Response (existing canonical-hit path unchanged; NEW fields when LLM path fires):
```json
{
  "matched": false,
  "source": "llm",
  "answer": "Pricing scales linearly per worker — at 50 workers the seat tier kicks in...",
  "declared_claim_classes": ["pricing"],
  "disposition": "Allow" | "Escalate" | "Block",
  "block_reason": null,
  "channel": "WebEmbed",
  "latency_ms": 850,
  "model": "claude-sonnet-4-6",
  "tokens": { "input": 412, "output": 87 }
}
```

When LLM detected `Block` disposition (forbidden claim class for this channel):
```json
{
  "matched": false,
  "source": "llm_blocked_canonical_refused",
  "answer": "<canonical refusal text from corpus, e.g. valuation-deferral>",
  "draft": "<the LLM's actual unsafe draft, redacted in production, logged for audit>",
  "block_reason": "forbidden_class: valuation",
  ...
}
```

### 2. Pipeline (~150 LOC new)

```rust
async fn pitch_ask(req: PitchAskReq, channel: ChannelKind) -> PitchAskResp {
    // Hot path: canonical-QA lookup — sub-millisecond, no cost
    if let Some(hit) = canonical_qa.lookup(&req.question) {
        let claims = hit.declared_claim_classes.iter()
            .map(|c| ClaimCandidate { class: c.clone(), text: hit.answer.clone() });
        let disposition = evaluate_max(claims, policy_for(channel));
        return PitchAskResp::canonical(hit, disposition);
    }

    // Fallback: LLM path with truth-boundary in system prompt
    let system = build_pitch_system_prompt(channel, &company_facts());
    let messages = vec![user_message(&req.question)];
    let llm_resp = provider_call(model, system, messages, max_tokens=500).await?;

    // Extract claim candidates from the LLM's response
    let candidates = extract_claim_candidates(&llm_resp.text, &policy_for(channel));
    let disposition = evaluate_max(candidates, policy_for(channel));

    match disposition {
        Block(class) => {
            // LLM tried to make a forbidden claim — swap in canonical refusal
            let refusal = canonical_qa.lookup_refusal_for(class)
                .unwrap_or_else(|| default_refusal_for(class));
            PitchAskResp::llm_blocked_canonical_refused(llm_resp, refusal, class)
        }
        Allow | Escalate => PitchAskResp::llm(llm_resp, disposition),
    }
}
```

### 3. System prompt structure (the truth-boundary in context)

The LLM system prompt for Pitch Agent includes:

- **Persona**: "You are the SKYCOT Pitch Agent. You represent the company in investor and customer conversations."
- **Channel context**: "You are speaking on the {channel} channel. {channel-specific tone notes}."
- **Truth boundary policy** (rendered from `policies/truth-boundary/skycot-{public-pitch|private-rehearsal}.toml`):
  - Forbidden claim classes for this channel (e.g. `valuation`, `unannounced-roadmap`, `revenue` on WebEmbed)
  - The canonical refusal text the agent should USE if the operator asks about a forbidden topic ("I'm not authorized to share valuation publicly. Mark, do you want to take that one?")
- **Company facts** (loaded from `packs/pitch-agent/facts.toml` or equivalent — operator-reviewed):
  - Founder name, locale, mission, key product invariants, current customer count tier (NOT specific count), pricing model shape (NOT specific numbers for forbidden classes), competitive positioning
- **Output discipline**:
  - "Answer in 1-3 sentences. Be evidence-led, not aspirational. Cite facts only from the approved set above."
  - "If you don't know an answer or it would require a forbidden claim, use the canonical refusal text verbatim and offer to escalate to {founder_name}."

### 4. Post-LLM claim extraction

Lightweight extractor in `argo-capability::evaluate_claim` mode. For v1:
- Pattern-match common forbidden phrasings ("we are valued at", "our revenue is", "Q3 roadmap includes")
- Per-class extractors loaded from policy TOML
- If LLM's text triggers a forbidden pattern → Block
- v2 could add a small classifier model; v1 stays regex-based

### 5. Audit emission

Every LLM-path request emits TWO audit rows:
- `pitch.llm_call_emitted` — model, tokens, latency, response_hash
- `pitch.truth_boundary_evaluated` — disposition, block_reason, swapped_response (bool)

Canonical-hit path stays as-is (single `pitch.canonical_qa_matched` row).

### 6. SPA changes (minimal)

`/pitch` page already shows: ALLOWED/ESCALATE/BLOCK pill + entry_id chip + latency badge. ADD:

- `source` chip — "canonical" (existing) or "llm" (new, with model name + token count tooltip on hover)
- When BLOCKED+canonical-swap fires, show the swap inline: "(canonical refusal substituted)" as a sub-line

~80 LOC TypeScript on the existing page component.

### 7. New pitch-eval cases

Extend the eval suite with:
- Novel-question cases (not in corpus) where LLM should produce a reasonable Allow response
- Adversarial cases where LLM might want to mention valuation/revenue/roadmap → must Block and swap to refusal
- Hot-path-still-works cases (regression — top-15 canonical questions still hit canonical path, not LLM)

Target: 30 new cases. Total eval count: 111 + 30 = 141.

## Acceptance

- [ ] `POST /api/v1/pitch/ask` with novel question returns LLM-sourced answer in <2s
- [ ] `POST /api/v1/pitch/ask` with forbidden-claim-triggering novel question returns canonical refusal + Block disposition + draft logged
- [ ] `POST /api/v1/pitch/ask` with canonical-hit question still returns in <5ms (regression — hot path untouched)
- [ ] Truth-boundary swap audited in `/audit` feed
- [ ] SPA `/pitch` page renders source chip + token count on LLM responses
- [ ] `pitch-eval` regression: 111 + 30 new cases all green
- [ ] Provider routing: by default uses the active provider manifest (currently anthropic.toml). Once `claude-code-cli` provider lands (sibling brief `provider-claude-code-cli-subprocess`), no code change here — just routing-config flip.

## Out of scope

- Multi-turn conversation memory (single-shot Q&A only for v1)
- Streaming the LLM response (instant response, accept the latency)
- Per-tenant company-facts override (v1 = single Skycot facts; v1.1 = per-tenant)
- Replacing canonical-QA with LLM-only (canonical-QA stays as load-bearing hot path)

## May-touch

- **Rust**: `crates/argo-cli/src/handlers/gateway/routes_pitch_ask.rs` (extend), `routes_pitch_ask_tests.rs` (extend), new `routes_pitch_ask_llm.rs` (LLM path module), `crates/argo-capability/src/` (claim-pattern extractor)
- **Rust new**: `crates/argo-eval/src/pitch_runner.rs` (extend with LLM-path cases), `pitch_runner_tests.rs` (extend)
- **Web**: `web/app/(app)/pitch/page.tsx` (source chip + token count display), `page.test.tsx` (regression + new tests), `web/lib/api/pitch.ts` (response type extension)
- **Content**: `packs/pitch-agent/facts.toml` (NEW — company facts loaded into LLM context)
- **Policies**: `policies/truth-boundary/skycot-*.toml` (extend with per-class refusal canonical text references — already shaped right per #2295)

## Operator overnight authority

Per 2026-05-29T22:30Z grant: `--skip-clippy` lane authorized; full worker-side verification (clippy + tests + pitch-eval 111+30 = 141/141) required before merge.

LLM provider routes via the active `Provider` trait — if `provider-claude-code-cli-subprocess` brief ships first, this brief uses it; if not, uses `anthropic.toml` (the current live route).
