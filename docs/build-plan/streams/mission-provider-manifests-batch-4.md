# Mission load-bearing wire — 4 new provider TOML manifests (cohere + mistral + groq + perplexity)

**Owner**: claude. **Depends on**: existing `manifests/providers/anthropic.toml` + `openai.toml` + `gemini.toml` as canonical TOML-manifest shape (verify via `ls manifests/providers/ && cat manifests/providers/anthropic.toml | head -40`).
**Authored against**: origin/main `281508ec` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-provider-manifests-batch-4`.

> SOURCE: MISSION_BACKLOG.md Section 5 #161 ("Ship 16+ providers via TOML manifests"). **Tier A (provider expansion).** Today SKYCOT ships ~5 provider manifests. The MISSION_BACKLOG target is 16+. This brief lands a focused batch of 4 well-scoped manifests: **Cohere** (enterprise-RAG), **Mistral** (EU-hosted, GDPR-friendly), **Groq** (LPU-fast inference), **Perplexity** (online-with-citation). Each is an OpenAI-compatible-ish chat API with minor shape variations — TOML-manifest-only, no Rust. Brings the total to ~9.

## Scope

NEW 4 TOML manifest files in `manifests/providers/`:

### `manifests/providers/cohere.toml` (≤ 90 LOC)
- API base: `https://api.cohere.com/v2`
- Auth: `Authorization: bearer <api_key>` (env: `COHERE_API_KEY`)
- Chat endpoint: `POST /chat`
- Models: command-r-plus, command-r, command-r-08-2024 (current generation)
- Pricing: per-model `input_micros_per_token` + `output_micros_per_token` (current rates as of brief author date)
- Stream: SSE `event: message` events
- Cohere-specific quirk: `tool_calls` field uses `tool_plan` + `tool_calls` shape (slightly different from OpenAI's tool_calls)

### `manifests/providers/mistral.toml` (≤ 80 LOC)
- API base: `https://api.mistral.ai/v1`
- Auth: `Authorization: Bearer <api_key>` (env: `MISTRAL_API_KEY`)
- Chat endpoint: `POST /chat/completions` (OpenAI-compatible)
- Models: mistral-large-latest, mistral-medium-latest, mistral-small-latest, codestral-latest
- Pricing: per-model rates
- EU-hosted note: data residency in EU (Paris); document for GDPR-conscious operators
- Stream: OpenAI-compatible SSE
- JSON mode: `response_format: { type: "json_object" }` supported

### `manifests/providers/groq.toml` (≤ 80 LOC)
- API base: `https://api.groq.com/openai/v1`
- Auth: `Authorization: Bearer <api_key>` (env: `GROQ_API_KEY`)
- Chat endpoint: `POST /chat/completions` (OpenAI-compatible)
- Models: llama-3.3-70b-versatile, llama-3.1-8b-instant, mixtral-8x7b-32768, gemma2-9b-it
- Pricing: per-model rates (Groq is significantly cheaper than OpenAI for the same Llama models)
- Latency note: LPU-based; sub-100ms time-to-first-token typical
- Stream: OpenAI-compatible SSE
- Rate limits: aggressive — document the `RPM` + `TPM` headers

### `manifests/providers/perplexity.toml` (≤ 80 LOC)
- API base: `https://api.perplexity.ai`
- Auth: `Authorization: Bearer <api_key>` (env: `PERPLEXITY_API_KEY`)
- Chat endpoint: `POST /chat/completions` (OpenAI-compatible-ish)
- Models: llama-3.1-sonar-large-128k-online, llama-3.1-sonar-small-128k-online, llama-3.1-sonar-huge-128k-online
- Pricing: per-model rates; online models cost more (citation-retrieval overhead)
- Online-with-citation quirk: responses include `citations: [url, ...]` field — operators using verticals that need source attribution (research-analyst, lead-research) should prefer this provider
- Stream: OpenAI-compatible SSE

This is NOT in scope: full integration / chat-loop validation against any of these providers (that's runtime-level integration smoke, deferred to operator-side env-gated tests); per-model context-window limits beyond what's documented in the manifest (the model serves that metadata at runtime); function-calling-shape parity with OpenAI (each provider's tool-calls schema varies; document the difference in a comment in the manifest); fine-tuning endpoints; embedding endpoints (separate brief once a vertical needs them); image-generation endpoints (covered by mission-image-generation-mcp-server #1326).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `manifests/providers/cohere.toml` (NEW, ≤ 90 LOC)
- `manifests/providers/mistral.toml` (NEW, ≤ 80 LOC)
- `manifests/providers/groq.toml` (NEW, ≤ 80 LOC)
- `manifests/providers/perplexity.toml` (NEW, ≤ 80 LOC)
- `docs/skycot/providers.md` (extend if exists; new if missing, ≤ 200 LOC) — per-provider operator notes: data residency / pricing tier / latency profile / online-or-cached / known quirks

## Files you must NOT touch

- Any `crates/**` Rust source — manifests are loaded by existing `argo-provider::manifest` parser; no code change required
- Existing manifests (`anthropic.toml`, `openai.toml`, `gemini.toml`, etc.) — sibling-stable; mirror their shape, don't modify them
- `crates/argo-store/migrations/**`
- Vertical content / channels

## Acceptance criteria

- [ ] `argo provider list` includes the 4 new providers (cohere, mistral, groq, perplexity)
- [ ] `argo provider show cohere --json` returns the parsed manifest with all model entries
- [ ] Same `argo provider show <name> --json` works for mistral, groq, perplexity
- [ ] Each manifest validates against the canonical TOML schema (compare to `anthropic.toml`'s shape — keys are: `[provider]`, `[[models]]`, `[transport]`, `[pricing]` or whatever the live shape is)
- [ ] No model declares an inconsistent pricing (`output_micros_per_token` should be ≥ `input_micros_per_token` for most providers; if exception, document in a manifest comment)
- [ ] Each provider has at least 1 streaming-capable model marked as such
- [ ] Operator-facing doc covers all 4 providers w/ data-residency notes (Mistral EU is the key one)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (no code change but verify the manifest-parsing logic doesn't complain about the new files at load time)
- [ ] `cargo test -p argo-provider` passes (provider manifest tests should pick up new manifests automatically if there's a discovery test)
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-provider-manifests-batch-4`
- PR title: `provider: 4 new manifests (cohere + mistral + groq + perplexity) (mission-provider-manifests-batch-4)`

## Notes for the implementer

- First read `manifests/providers/anthropic.toml` AND `openai.toml` AND `gemini.toml` to learn the canonical TOML shape. Match it exactly — don't invent a new shape for the new providers.
- Cohere v2 API: the chat shape is OpenAI-compatible-with-quirks. The `tool_plan` field is a free-text reasoning summary; the actual tool calls are in `tool_calls`. Document this in the manifest comment.
- Mistral's `mistral-large-latest` is the flagship; document data-residency (EU Paris) prominently — that's the main reason an operator picks Mistral over GPT-4o.
- Groq's models are all open-weight (Llama, Mixtral, Gemma) — same MODEL but VASTLY faster inference via LPU hardware. Operators picking Groq optimize for latency, not capability.
- Perplexity's `citations` field in the response is the differentiator. Verticals that need source attribution (research-analyst, lead-research, legal-paralegal) get free Bibliography-quality output. Document this.
- Pricing values: best-effort current as of brief-author date. The TOML allows operators to override per-tenant; that's the dynamic-pricing seam.
- Don't ship a manifest for a model that's been deprecated (e.g. Mistral's `mistral-tiny` is EOL'd). Stick to currently-published models.
- For the operator-facing doc, the comparison table should be:
  - Provider | Data Residency | Latency | Online/Cached | Best For
  - Cohere | US | medium | cached | enterprise RAG
  - Mistral | EU | medium | cached | GDPR-conscious
  - Groq | US | very-fast | cached | low-latency
  - Perplexity | US | medium | online-with-citation | research / fact-grounded
- DO NOT try to add a sixth manifest for "completeness" — this brief is scoped to 4. Each additional manifest is its own brief.
