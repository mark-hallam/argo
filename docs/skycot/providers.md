# Provider catalog — operator notes

Per-provider operator notes for the TOML manifests bundled under
`manifests/providers/`. The TOML files are the source of truth for
wire-protocol details (endpoints, auth, tool-call format, models,
pricing); this doc captures the human-readable context that doesn't
fit in a manifest comment: data residency, latency profile, the
fetch / cache / online distinction, and per-provider quirks the
runtime ignores but operators need to know about.

For the canonical TOML shape, see `crates/argo-provider/src/manifest/mod.rs`
and the existing `anthropic.toml` / `openai-compatible.toml` examples.

## Comparison table

| Provider    | Data Residency | Latency    | Online/Cached         | Best For                      |
| ----------- | -------------- | ---------- | --------------------- | ----------------------------- |
| Anthropic   | US             | medium     | cached                | reasoning-heavy, long context |
| OpenAI      | US (multi)     | medium     | cached                | general purpose               |
| Bedrock     | configurable   | medium     | cached                | AWS-resident workloads        |
| Cohere      | US             | medium     | cached                | enterprise RAG                |
| Mistral     | EU (Paris)     | medium     | cached                | GDPR-conscious                |
| Groq        | US             | very-fast  | cached                | low-latency                   |
| Perplexity  | US             | medium     | online-with-citation  | research / fact-grounded      |

"Latency" rates the typical time-to-first-token for the provider's
flagship model on a normal-sized prompt; it's a coarse signal, not an
SLA. "Online" providers fetch web content per request and return
source URLs — counts as live retrieval, not a static-weights model.

## Cohere

- **Manifest**: `manifests/providers/cohere.toml`
- **API base**: `https://api.cohere.com`
- **Chat endpoint**: `POST /v2/chat`
- **Auth env**: `COHERE_API_KEY` (sent as `Authorization: bearer <key>`)
- **Tool-call shape**: OpenAI-compatible `tool_calls` array, plus a
  free-text `tool_plan` reasoning summary that Cohere returns alongside
  the structured tool calls. Tool dispatch reads `tool_calls`;
  `tool_plan` is advisory (log it for observability, drop it for
  downstream parsing).
- **Models bundled**: `command-r-plus` (flagship), `command-r`,
  `command-r-08-2024`.
- **Best for**: enterprise-grade RAG where the model's grounding and
  citation behavior is more important than raw reasoning depth.
- **Streaming**: SSE, `event: message` framing.

## Mistral

- **Manifest**: `manifests/providers/mistral.toml`
- **API base**: `https://api.mistral.ai`
- **Chat endpoint**: `POST /v1/chat/completions` (OpenAI-compatible)
- **Auth env**: `MISTRAL_API_KEY` (sent as `Authorization: Bearer <key>`)
- **Data residency**: **EU Paris**. This is the load-bearing reason
  to pick Mistral over GPT-4o for GDPR-sensitive workloads — model
  capability is comparable for most tasks; data location is the
  differentiator.
- **JSON mode**: supported via `response_format = { type = "json_object" }`
  in the request body. Useful for structured-output verticals
  (intake forms, ticket extraction, lead-qualification fields).
- **Models bundled**: `mistral-large-latest` (flagship),
  `mistral-medium-latest`, `mistral-small-latest`, `codestral-latest`
  (code-specialized).
- **Best for**: any vertical with EU-resident customers or contracts
  that require non-US-hosted inference.
- **Streaming**: SSE, OpenAI-compatible framing.

## Groq

- **Manifest**: `manifests/providers/groq.toml`
- **API base**: `https://api.groq.com`
- **Chat endpoint**: `POST /openai/v1/chat/completions` (OpenAI-compatible)
- **Auth env**: `GROQ_API_KEY` (sent as `Authorization: Bearer <key>`)
- **Models bundled**: `llama-3.3-70b-versatile`, `llama-3.1-8b-instant`,
  `mixtral-8x7b-32768`, `gemma2-9b-it` — all open-weight, hosted
  on Groq's custom LPU hardware.
- **Latency**: sub-100ms time-to-first-token typical. Operators
  routing on TTFT alone will see large wins switching from
  GPU-hosted Llama to Groq-hosted Llama for the same model id.
- **Rate limits**: aggressive on the free tier. Response carries
  `x-ratelimit-{limit,remaining}-{requests,tokens}` headers; the
  runtime can use these for budget-aware fallback.
- **Best for**: latency-critical interactive verticals (live receptionist,
  realtime translation, voice-assistant prompts) where TTFT beats
  raw capability.
- **Streaming**: SSE, OpenAI-compatible framing.

## Perplexity

- **Manifest**: `manifests/providers/perplexity.toml`
- **API base**: `https://api.perplexity.ai`
- **Chat endpoint**: `POST /chat/completions` (OpenAI-compatible-ish)
- **Auth env**: `PERPLEXITY_API_KEY` (sent as `Authorization: Bearer <key>`)
- **Models bundled**: `llama-3.1-sonar-large-128k-online`,
  `llama-3.1-sonar-small-128k-online`,
  `llama-3.1-sonar-huge-128k-online`. All `*-online` models perform
  live web retrieval per request.
- **Citations**: every response includes a `citations: [url, ...]`
  array adjacent to the message content. Verticals doing research,
  fact-grounded summaries, or anything where the user needs source
  attribution should prefer Perplexity for free bibliography-quality
  output.
- **Pricing**: online models cost more than equivalent-size offline
  Llama deployments — citation-retrieval overhead is bundled.
- **Best for**: research-analyst, lead-research, legal-paralegal, and
  any vertical that wants live web data with provenance.
- **Streaming**: SSE, OpenAI-compatible framing.

## Configuring API keys

API keys are read from the environment vars declared in each
manifest's `[auth].api_key_env` field. The capability-engine grants
the corresponding `EnvRead` permission per Invariant 3; if a key env
var is unset at runtime the provider transport refuses the call with
an `ArgoError::Provider` rather than dialing the endpoint.

See `docs/skycot/provider_keys.md` for the recommended `direnv` /
secrets-manager pattern.

## Adding a new provider manifest

1. Read `manifests/providers/anthropic.toml` and
   `manifests/providers/openai-compatible.toml` to learn the canonical
   shape.
2. Pick the closest `family =` value (`openai-compatible`,
   `openai-responses`, `anthropic`, or `bedrock`) — the family
   selects which Rust transport implementation handles the wire.
3. Author the TOML under `manifests/providers/<id>.toml`. Keep it
   under 500 LOC.
4. Add an entry to this doc with data-residency / latency / best-for
   notes.
5. `argo provider validate` must pass.
6. If the manifest needs a wire shape no existing family covers,
   the change is a Rust change (new `ProviderFamily` variant + new
   transport crate), not a manifest-only change.
