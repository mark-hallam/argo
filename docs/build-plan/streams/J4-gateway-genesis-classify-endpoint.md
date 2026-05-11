# Stream J4 — Gateway Genesis endpoints (`/api/v1/genesis/classify` + `/draft-worker`)

**Owner**: claude. **Depends on**: existing Genesis backend (`mission-genesis-llm-classifier` MERGED + `mission-genesis-end-to-end-smoke` MERGED).
**Authored against**: origin/main `b36e81dc` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J4-gateway-genesis-classify-endpoint`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 4 of 18 — unblocks frontend E2-c + E2-d from stub Genesis fallback.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Auth | Bearer token (E0 precedent) | Same; SSO is the auth-bridge brief | `mission-enterprise-auth-sso` |
| Tenant scoping | Calls Genesis with caller's tenant_id; Genesis filters skills/templates by tenant | Same | n/a |
| Cost tracking | Per-call LLM cost reported in response (`cost_usd` field) | Per-tenant cumulative cost cap with kill switch | `mission-enterprise-genesis-cost-cap` |
| Rate limiting | None at v1 (Genesis itself enforces no rate limit) | Per-user + per-tenant hire-flow rate limit | `mission-enterprise-genesis-rate-limit` |
| Content moderation | None at v1 | Server-side moderation on generated questions + worker draft | `mission-enterprise-genesis-output-moderation` |
| Audit emission | Every Genesis call emits an audit row | Same | n/a |

v1 is acceptable: Genesis backend already enforces
tenant isolation; the HTTP wrapper just plumbs the
auth context through.

## Why

E2-c (Step 2 Listen) calls Genesis-lite via
`POST /api/v1/genesis/classify` to generate
clarifying questions. E2-d (Step 3 Drafting) calls
`POST /api/v1/genesis/draft-worker` to generate the
full worker draft. Until these endpoints exist, both
frontend briefs fall back to client-side stub
generators.

This brief lands both endpoints so production hire-
flow actually drives a real Genesis pipeline.

## Scope

- 2 JSON endpoints under `/api/v1/genesis/`:
  - `POST /api/v1/genesis/classify` body
    `{ problem: string }` →
    `{ classification: string, questions: Question[], cost_usd?: number }`.
  - `POST /api/v1/genesis/draft-worker` body
    `{ problem, answers, adjustment? }` →
    `{ worker_draft: WorkerDraft, cost_usd?: number }`.
- Both call into the existing Genesis pipeline in
  `crates/argo-genesis` (per the merged briefs).
- Bearer-auth required; tenant from Principal.
- 30s server timeout (Genesis LLM calls vary; long
  ones return 408 to let the client retry).
- Audit row per call with cost metadata.
- 4xx error envelope on validation failures (e.g.
  empty problem string, invalid answer shape).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  - `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`
    (POST dispatch — narrowly add 2 arms)
  - `crates/argo-cli/src/handlers/gateway/mod.rs`

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_genesis.rs`
  — NEW. 2 endpoint handlers + request/response
  type defs. ≤ 350 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_genesis_tests.rs`
  — NEW. ≥ 10 unit tests. ≤ 400 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly scoped: 1-line `pub mod` addition.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`
  — narrowly scoped: 2 arms (classify + draft-worker).
  ≤ 280 LOC after.
- `docs/skycot/RUNBOOK.md` — narrowly scoped: add
  §"Genesis API" with endpoint specs + curl
  examples.

## Files you must NOT touch

- `crates/argo-genesis/**` — read-only. The
  pipeline already exists from the merged briefs;
  this brief is a thin HTTP wrapper.
- Other Track-J handler files.
- Frontend `lib/api/genesis.ts` — locked from
  E2-c.

## Acceptance criteria

- [ ] `POST /api/v1/genesis/classify` with valid
      body returns 200 + the question set + cost.
- [ ] `Question` JSON shape matches frontend's
      `web/lib/hire/genesis-types.ts`:
      ```json
      {
        "id": "q1",
        "text": "...",
        "answer_type": "single-select" | "multi-select" | "free-text" | "yes-no",
        "options": ["..."],
        "required": true
      }
      ```
- [ ] `POST /api/v1/genesis/draft-worker` returns
      a `WorkerDraft` matching frontend
      `web/lib/hire/draft-types.ts` (suggestedName +
      persona + charter + duties[] + exampleReplies[]
      + approvalPolicy + suggestedVertical).
- [ ] Empty problem string → 400 with envelope
      `{ error: { code: "bad_request", message: "..." } }`.
- [ ] Missing answers in draft-worker for required
      questions → 400.
- [ ] Genesis pipeline error → 500 with envelope.
- [ ] 30s server timeout returns 408 Request
      Timeout.
- [ ] Both endpoints require Bearer auth (401
      otherwise).
- [ ] Tenant scoping verified: Genesis sees
      caller's tenant_id via the existing
      pipeline integration.
- [ ] Per-call cost reported in response
      (`cost_usd: <number>`); if Genesis doesn't
      surface cost yet, response includes
      `cost_usd: null`.
- [ ] Audit row written per call with metadata
      `{ endpoint, problem_length, question_count, cost_usd }`.
- [ ] 10 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J4-gateway-genesis-classify-endpoint`
- PR title: `gateway: Genesis classify + draft-worker endpoints (J4)`

## Notes for the implementer

- **Genesis pipeline integration.** The existing
  Genesis crate exposes (per the merged briefs):
  - `argo_genesis::classify(problem,
    tenant_id) -> ClassifyResult`
  - `argo_genesis::draft_worker(problem, answers,
    tenant_id) -> WorkerDraft`
  Check the actual function signatures by reading
  `crates/argo-genesis/src/lib.rs` first; this
  brief assumes the rough shape.

- **Adjustment param.** `draft-worker` accepts an
  optional `adjustment: string` from the frontend's
  Adjust drawer (E2-e). When present, Genesis re-
  runs with the user's free-text adjustment as
  additional context. Pass it through to the
  pipeline.

- **30s timeout.** Use `tokio::time::timeout` to
  bound the Genesis call. On timeout: return 408
  with body `{ error: { code: "timeout", message:
  "Genesis is taking longer than usual. Try again."
  } }`. Don't crash the Tokio runtime.

- **Cost reporting.** If the underlying LLM call
  reports `cost_usd` (it should, per the prompt-
  cache-hit-rate-observability brief), bubble it
  up. If not (older Genesis path), set to null.
  Frontend handles either gracefully per E2-c's
  cost-reporting-forward-compatibility note.

- **Idempotency.** Genesis is non-deterministic
  (LLM calls vary). Two identical requests CAN
  produce different outputs. Frontend handles via
  optimistic UX + Adjust retry; backend doesn't
  need idempotency keys for v1.

- **Validation rules:**
  - `problem.trim().len() >= 15` (matches
    frontend's E2-b validation).
  - `answers` shape matches the questions from
    a prior classify call (server doesn't store
    the question set; client passes it back). If
    answer for a required question is missing →
    400.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-genesis-cost-cap` — per-
  tenant cumulative cost cap with kill switch.
- `mission-enterprise-genesis-rate-limit` —
  per-user + per-tenant rate limit.
- `mission-enterprise-genesis-output-moderation`
  — server-side moderation on questions + draft.
