# Genesis API

Two JSON endpoints back the SPA "hire a worker" flow (Track E2-c
+ E2-d). v1 is a thin HTTP wrapper around the existing
`argo-genesis` pipeline; the keyword classifier runs offline so
cost is reported as `null`.

## `POST /api/v1/genesis/classify`

**Body**

```json
{ "problem": "Marketing intern to handle our weekly newsletter" }
```

`problem` is required, must be at least 15 trimmed characters.

**200 response**

```json
{
  "classification": "marketing_intern",
  "questions": [
    { "id": "role", "text": "...", "answer_type": "free-text", "required": true },
    { "id": "name", "text": "...", "answer_type": "free-text", "required": true },
    { "id": "industry", "text": "...", "answer_type": "free-text", "required": false },
    { "id": "duties", "text": "...", "answer_type": "free-text", "required": true },
    { "id": "channels", "text": "...", "answer_type": "multi-select",
      "options": ["slack-dm", "email", "sms", "discord", "whatsapp"], "required": true },
    { "id": "spend_cap", "text": "...", "answer_type": "free-text", "required": true }
  ],
  "cost_usd": null
}
```

`classification` is one of the 12 archetype slugs or `"unknown"`
when no keyword matches. `cost_usd` is `null` in v1 — the
hardening brief `mission-enterprise-genesis-cost-cap` wires
per-tenant cost tracking.

**Errors**

- `400 bad_request` — body parse error, missing `problem`, or
  `problem` shorter than 15 trimmed characters.
- `408 timeout` — Genesis pipeline exceeded 30s (LLM-classifier
  rollout safety net; the v1 keyword path completes in
  microseconds).

**Example**

```bash
curl -sS -X POST http://localhost:8080/api/v1/genesis/classify \
  -H "Authorization: Bearer $ARGO_GATEWAY_BEARER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"problem":"Customer support agent for our SaaS billing tickets"}'
```

## `POST /api/v1/genesis/draft-worker`

**Body**

```json
{
  "problem": "Marketing intern to handle our weekly newsletter",
  "answers": {
    "role": "marketing intern",
    "name": "Maya",
    "industry": "B2B SaaS",
    "duties": ["draft newsletter", "post to LinkedIn", "qualify leads"],
    "channels": ["slack-dm", "email"],
    "spend_cap": 50
  },
  "adjustment": "Avoid emailing leads on weekends."
}
```

`answers` is the operator's responses to the question set returned
by `/classify`. `adjustment` is optional free text appended to the
charter (used by the Adjust drawer in E2-e). `duties` and
`channels` accept either a JSON array or a delimited string for
forward-compat with pre-E2-d clients.

**200 response**

```json
{
  "worker_draft": {
    "suggestedName": "Maya",
    "persona": "maya",
    "charter": "...full rendered charter...",
    "duties": ["draft newsletter", "post to LinkedIn", "qualify leads"],
    "exampleReplies": ["Sure — I can ...", "..."],
    "approvalPolicy": "5 of 8 actions require operator approval. ...",
    "suggestedVertical": "marketing_intern_maya"
  },
  "cost_usd": null
}
```

**Errors**

- `400 bad_request` — body parse error, missing `problem` /
  `answers`, missing required answer (`role`, `name`, `duties`,
  `channels`, `spend_cap`), non-positive `spend_cap`.
- `408 timeout` — Genesis pipeline exceeded 30s.
- `500 internal` — Genesis pipeline error (rare; usually means an
  invalid persona slug or template-loader failure).

**Auth + audit**

Both endpoints require the same bearer token as every other
`/sessions*` write. Every successful call writes one
`http.request` audit row with the Genesis-specific payload
`{endpoint, problem_length, question_count, cost_usd}` so
operators can replay hire-flow attempts via `argo audit query`.
