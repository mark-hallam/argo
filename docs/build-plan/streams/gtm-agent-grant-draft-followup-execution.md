# gtm-agent-grant-draft-followup-execution

**Priority**: P0 — the "teammate does real work" proof for Grant (the GTM Engineer). Grant takes on a RESPONSIBILITY (own inbound growth) and does real, brand-safe work: given a waitlist signup, he drafts a personalized outreach follow-up via the real LLM, truth-bounded, and routes it to the approval queue. Recursive proof: Skycot's GTM teammate growing Skycot.
**Source**: 2026-05-29 operator direction (GTM teammate as the 2nd real working teammate after Portia). Mission: "teammates take on responsibilities, not tasks" (Claim 2).
**Authored against**: origin/main `abfc751e8` on 2026-05-29
**Authored by**: claude conductor (interactive session)

## Why this brief exists (Phase B)

Phase A shipped Grant as the 7th hero teammate (seed persona, vertical `gtm`, charter "own inbound growth", Growth cohort). But — like the other static teammates — Grant currently *does nothing*. The mission is teammates that **do real work**. Portia proves the pattern (real LLM via the operator's Claude subscription + truth-boundary + audit). This brief extends that exact pattern to Grant's first real responsibility.

## The one real responsibility (first slice)

**Inbound waitlist follow-up drafting**, approval-gated:
1. A waitlist signup arrives (`{ name, email, context? }`).
2. Grant drafts a personalized, **truth-bounded** follow-up via the real LLM.
3. The draft + provenance lands in the **approval queue** (`/approvals`) + an **audit row** — never auto-sent. The operator approves → (real deploy) sends.

This is a **responsibility** (standing duty, judgment, escalation, approval) not a task. Safe (drafts only), real (live LLM), recursive (grows Skycot).

## Scope — reuse Portia's primitives, do NOT rebuild

The pitch endpoint already does LLM-via-claude-code-cli + truth-boundary + audit. Mirror it:

1. **`crates/argo-cli/src/handlers/gateway/routes_gtm_draft_followup.rs` (NEW)** — `POST /api/v1/gtm/draft-followup`. Body: `{ signup: { name, email, context? } }`. Behavior:
   - Build the prompt from Grant's charter + the shared Skycot facts (`packs/pitch-agent/facts.toml` — reuse, or a `packs/gtm-agent/facts.toml` that includes it) + the signup context. System prompt instructs: write a short, warm, specific follow-up; only claim approved facts; if a personalized hook needs info not present, keep it generic.
   - Draft via the **same transport** as pitch: `build_pitch_transport("claude-code-cli")` (DEC-012; Sonnet floor DEC-013) — generalize/rename the helper if cleaner (`build_llm_transport`).
   - Run the draft through `argo_capability::evaluate_claim` (truth-boundary): `Allow` → keep; `Escalate`/`Block` → swap in a safe escalation note ("flagging this lead for founder follow-up").
   - **Write an `approval_request`** (so it surfaces in `/approvals`) describing the drafted outbound, tenant-bound, with the draft as the body + provenance (source=llm, model, disposition). Plus a `gtm.followup_drafted` **audit row**.
   - Return `{ draft, disposition, model, source, approval_id }`.
   - Gate the LLM the same way pitch does (`ARGO_PITCH_LLM_FALLBACK`/provider env, or a `ARGO_GTM_LLM_*` equivalent).
2. **Register the route** in the gateway dispatch (mirror where `pitch/ask` is mounted).
3. **SPA surface (one of):**
   - simplest: Grant's drafts appear in the existing **`/approvals`** queue (no new page) — the operator approves there. *(Preferred for v1 — reuses Scene 6.)*
   - optional later: a small `/gtm` console (mirror `/pitch`) where you paste a signup and watch Grant draft live.
4. **Tests:** unit test the prompt build + the truth-boundary swap (mirror `routes_pitch_ask_llm_tests.rs`); an integration test that a draft writes an `approval_request` + audit row, tenant-scoped.

## Acceptance

- [ ] `POST /api/v1/gtm/draft-followup` with a signup returns a real LLM-drafted follow-up (`source: llm`, `model: claude-sonnet-4-6`) routed through the truth-boundary.
- [ ] The draft appears as an `approval_request` in `/approvals` (tenant-bound) + a `gtm.followup_drafted` audit row exists — verifiable cold.
- [ ] A draft that would exceed approved facts → `Escalate`/`Block` swaps in the safe note (no hallucinated claims). Brand-safe like Portia.
- [ ] Routes via `claude-code-cli` (operator subscription), Sonnet floor, never Haiku.
- [ ] `cargo clippy -p argo-cli -p argo-capability -p argo-provider --all-targets -- -D warnings` clean; `cargo test` green incl. new tests + surface_contract if exports change.
- [ ] No `unwrap`/`expect` outside `#[cfg(test)]`; rustdoc on new pub items.

## Out of scope (later slices)

- Actually SENDING the email (deploy-time; demo is draft→approve only — NO THEATER means we don't fake a send).
- Real waitlist ingestion wiring (a signup webhook) — for the demo, drive it with a sample signup payload; real ingestion is a follow-up.
- Social-content drafting + weekly growth brief (Grant's other duties — separate slices).

## Operator authority

Rust + gateway + model calls → evaluator cold-cargo lane before merge (not the web fast lane). Reuses the proven Portia LLM+truth-boundary+audit primitives — this is "2nd vertical on the proven pattern," not new infrastructure.

## Integration map (discovered 2026-05-29 — makes this turnkey)

Exact files/APIs to wire (all confirmed against current main):

- **Route registration:** `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs:227` has the `pitch/ask` block — add a sibling:
  `if req.starts_with("POST /api/v1/gtm/draft-followup ") || req.starts_with("POST /api/v1/gtm/draft-followup?") { return Some(super::routes_gtm_draft_followup::draft_followup_response(req, store).await); }`
  + `mod routes_gtm_draft_followup;` in `mod.rs` (next to `routes_pitch_ask*` ~line 155-157).
- **LLM transport (REUSE):** `routes_pitch_ask_llm.rs::build_pitch_transport(provider_id)` (line 293) — make it `pub(super)`. Call `transport.complete(&ProviderRequest{ model, messages: vec![user_message(prompt)], system: Some(gtm_system_prompt), sampling: Sampling{ temperature: Some(0.3), max_tokens: Some(...), ..default }, tools: vec![] })`. Types from `argo_provider::{ProviderRequest, ProviderResponse, Sampling, Transport}`. `user_message(prompt)` helper (routes_pitch_ask_llm.rs ~308) — make `pub(super)` or inline it. Model: reuse `claude-sonnet-4-6` (Sonnet floor).
- **Truth-boundary (REUSE):** `routes_pitch_ask_llm.rs::extract_forbidden_class(draft)` (already `pub(super)`) → `Some(class)` ⇒ swap the draft for a safe escalation note + disposition `Escalate`/`Block`; `None` ⇒ `Allow`. (Or factor out `finalize_outcome`'s shaping.)
- **Approval write (REUSE):** `argo_store::{create_request, NewApprovalRequest}` (approvals_types.rs:110; the seed uses them at `seed.rs:11` + `seed_approval_requests` at `seed.rs:315` — copy that field-population pattern). Write the drafted follow-up as an `approval_request` (tenant via `rate_limit::extract_tenant(req)`; action_type e.g. `gtm.followup`; body = the draft + provenance). This is what surfaces it in `/approvals`.
- **Audit (REUSE — no new AuditKind):** use **`AuditKind::ApprovalAsked`** (Grant asks for sign-off on the draft) — avoids touching `event_kind.rs` + the `claim_6_approval_audit_kinds` variant-count test. Emit via the `AuditSink` pattern in `routes_pitch_ask.rs::emit_audit` (line 453 — make `pub(super)` or replicate the ~12 lines).
- **LLM gate:** `ARGO_PITCH_LLM_PROVIDER=claude-code-cli` + `ARGO_PITCH_LLM_FALLBACK=1` are already in `~/.argo/env.local`; reuse the same env (or add `ARGO_GTM_LLM_PROVIDER` falling back to it).
- **Tests:** mirror `routes_pitch_ask_llm_tests.rs` (prompt build + forbidden-swap, no live provider) + an integration test that a draft writes one tenant-scoped `approval_request` + `ApprovalAsked` audit row.

One rebuild (`restart-live-gateway.sh --force --no-prompt`) brings Grant live (he's seeded as the 7th hero on main already) + working. Verify: `curl POST /api/v1/gtm/draft-followup` returns a real `source:llm` draft; it appears in `/approvals`.
