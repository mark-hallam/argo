# fix-genesis-broad-role-graceful-on-non-json-refusal

**Owner**: claude.
**Status**: TODO — enqueued 2026-06-06 (conductor) from a live 20× e2e finding.
**Authored against**: origin/main `5b98f1fcf` on `2026-06-06`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-genesis-broad-role-graceful-on-non-json-refusal`.

## Why (live-reproduced 2026-06-06)

Driving `POST /api/v1/genesis` with a deliberately **destructive** role
("An ops assistant that deletes old customer records and clears server logs
every night… and wipes inactive accounts without asking") makes the underlying
LLM **refuse to synthesize** the role — it returns prose, not JSON. The
broad-role parser (`broad_role_parse.rs`) then errors and genesis emits a
**non-recoverable technical SSE error** at the first phase:

```
event: error
data: {"message":"genesis plan failed: refinement validation failed at
broad_role.synthesize: provider error: broad-role response: no JSON object",
"phase":"drafting_persona","recoverable":false}
```

Two problems:
1. **Ungraceful + confusing**: the user (or an attacker probing a dangerous
   role) gets a raw parser error, not a clean "this role can't be set up."
2. **Pre-empts the safety gate**: the failure is at the LLM-refusal layer,
   *before* genesis's own safety judge engages — so the safety machinery never
   gets to classify/reject it; genesis just crashes on the unparseable refusal.

Control: a benign role ("note-taking assistant that summarizes meeting notes")
synthesized + gated cleanly under identical load (rubric 0.765, safety allow),
so this is the **role/refusal path**, not provider quota.

## What

In the broad-role synthesize path, detect a **non-JSON / refusal** provider
response and degrade gracefully instead of a hard non-recoverable error:
- `broad_role_parse.rs` — when `first_balanced_object` / `parse_broad_role_response`
  finds no JSON object, distinguish "model refused / returned prose" from a
  transient parse glitch, and return a typed outcome the planner can handle
  rather than a bare `provider error`.
- `broad_role_plan.rs` (`plan_or_broad` / `GenesisOutcome`) — surface a
  **graceful, recoverable** outcome: a user-facing message like *"We couldn't
  set up this role — it may involve actions we can't safely automate. Try
  rephrasing without destructive/irreversible steps."* Prefer routing to the
  existing safety-rejection outcome shape so the gate's "what it will NOT do"
  framing is reused (no parallel message surface).
- SSE mapping (`routes_genesis_stream.rs`, if needed) — emit
  `recoverable:true` for this case so the SPA lets the user rephrase + retry
  rather than dead-ending.

Grep-before-write: extends the EXISTING `GenesisOutcome` / parse helpers — do
NOT add a parallel outcome enum. `rg 'pub enum GenesisOutcome' crates/argo-genesis`
before declaring anything.

## Verification
- Unit test in `broad_role_parse` / `broad_role_plan`: a prose/refusal response
  (no JSON object) → a graceful recoverable `GenesisOutcome`, NOT a hard
  `provider error`. A valid JSON response still parses unchanged.
- `cargo test -p argo-genesis` green + clippy clean.
- Manual: re-drive the destructive role through `/api/v1/genesis` → expect a
  clean recoverable rejection message, not `recoverable:false` "no JSON object".

## Out of scope
- Changing what the LLM refuses (that's the model's call). This is purely
  graceful HANDLING of a refusal/non-JSON response.
