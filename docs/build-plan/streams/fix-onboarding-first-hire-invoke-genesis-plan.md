# fix-onboarding-first-hire-invoke-genesis-plan

**Owner**: codex.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-first-hire-invoke-genesis-plan`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/onboarding-hire-an-ai-teammate.md` F5 + F9 + F10.

🔴 **P0 DEMO-BLOCKER — Scene 3 narrates "Skycot turns that sentence into a real vertical — system prompt, action vocabulary, memory schema, eval seeds." The first-hire endpoint never actually calls Genesis. The investor-demo claim that distinguishes Skycot from "ChatGPT with prompts" is fiction at this code path.**

## Root cause

`routes_onboarding::first_hire_response` (`crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:226-307`) is supposed to be the runtime endpoint backing the "AI Teammate Onboarding Studio" framing (per memory `feedback_genesis_is_ai_teammate_onboarding_studio`). Instead it:

1. Looks up an already-installed vertical id via `load_vertical(&vertical_id)` (the vertical was picked by the operator in step 4 — no synthesis).
2. Extracts a one-word name from the operator's prompt via `name_from_description` (covered separately by `fix-onboarding-first-hire-name-parser-prefers-named-pattern`).
3. Builds a `HireDraft` with `charter: format!("First hire for this workspace. Operator brief:\n\n{description}")` at line 262 — the charter is literally the operator's prompt with a 35-char preamble.
4. Inserts one hardcoded duty `"0 9 * * 1-5" daily_check_in` at lines 267-273 — the picked vertical's `sampleDuties` chip labels (e.g. "Draft blog posts / Schedule social posts / Weekly competitive scan") never materialise as actual scheduled work.
5. Calls `IdentityStore::create_worker_from_draft`.

`argo_genesis::plan()` is never invoked. No prompt synthesis, no action-vocabulary generation, no memory-schema derivation, no eval seeding. The runbook's referenced live-test source `crates/argo-workflow/tests/claim_3_live_genesis_to_run.rs` exercises Genesis through a different code path; the `/api/v1/onboarding/first-hire` surface the wizard hits does not.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F5 [P0] — Genesis not invoked on first-hire; "plain-English in → real teammate out" promise is fiction at this code path
- onboarding-hire-an-ai-teammate F9 [P2] — Charter is a templated 35-char-preamble + raw prompt instead of a worker-shaped persona
- onboarding-hire-an-ai-teammate F10 [P2] — Hardcoded `daily_check_in` duty; vertical's `sampleDuties` chips don't carry into the worker

## Fix recipe

1. **Add a Genesis invocation branch in `first_hire_response`**. After resolving the workspace tenant and before building the `HireDraft`:
   ```rust
   let plan = argo_genesis::plan(GenesisRequest {
       prompt: description.clone(),
       vertical_hint: Some(vertical_id.clone()),
       tenant_id,
       principal: principal.clone(),
   }).await?;
   ```

2. **Use the plan's outputs to populate the draft**:
   - `draft.name = plan.name` (overrides string-parsed name; respects operator-supplied `name` field if present)
   - `draft.charter = plan.charter` (synthesised, not templated)
   - `draft.tagline = plan.tagline`
   - `draft.voice = plan.voice`
   - `draft.duties = plan.duties` (one row per `sampleDuties` entry from the manifest, each with a `placeholder_cron` the operator can edit on the worker-detail Schedules tab — typically `"0 9 * * 1-5"` for the first, staggered for subsequent)
   - Persist eval seeds + memory schema via existing Genesis persistence path

3. **Define a deterministic fallback** for when `argo_genesis::plan()` fails (provider down, schema invalid, etc):
   - Emit an audit row tagged `genesis_fallback: true`
   - Fall through to today's templated-string behavior so the wizard never hard-fails mid-flow
   - Surface a banner on the worker-detail page: "Onboarding completed in fallback mode — re-run Genesis from Settings"

4. **Schema field for the picked vertical's `sampleDuties`**: derive duties from `vertical.sample_duties` (vertical-manifest field). If the field doesn't exist, add it as part of this brief.

5. **Audit row**: emit `onboarding.first_hire.genesis_invoked` with `{plan_id, model_id, charter_hash, duty_count}` so the camera-visible `/audit` row narrates "Skycot wrote this worker's charter at 14:32:11" in real time.

6. **Tests**:
   - `crates/argo-cli/tests/onboarding_first_hire_invokes_genesis.rs`: POST first-hire, assert `argo_genesis::plan` was called with the operator's prompt, assert the resulting worker has a charter that is NOT the templated `"First hire for this workspace..."` preamble.
   - Fallback test: stub the Genesis client to error, assert the wizard still completes and the audit row carries `genesis_fallback=true`.

## Acceptance criteria

- [ ] `POST /api/v1/onboarding/first-hire -d '{"description":"Maya is a marketing intern...","vertical_id":"marketing-intern"}'` triggers a Genesis invocation in logs.
- [ ] Worker's charter on `/api/v1/workers/<id>/overview` is the Genesis-synthesised string, not the `"First hire for this workspace..."` template.
- [ ] Worker has >=2 duties matching the picked vertical's `sample_duties`, not a single `daily_check_in`.
- [ ] An audit row with `kind = onboarding.first_hire.genesis_invoked` is emitted with `plan_id`, `model_id`, `charter_hash`.
- [ ] Fallback path: when Genesis errors, wizard still completes; audit row carries `genesis_fallback=true`; banner appears on worker detail.
- [ ] Cold integration test passes: `cargo test -p argo-cli onboarding_first_hire_invokes_genesis`.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` (the handler itself)
- `crates/argo-genesis/src/lib.rs` (verify `plan()` signature accepts what's needed; tweak if necessary)
- `crates/argo-vertical-pack/src/manifest.rs` (add `sample_duties` field if missing)
- `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` (fallback branch)
- `crates/argo-cli/tests/onboarding_first_hire_invokes_genesis.rs` (NEW)
- `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx` (banner for genesis_fallback worker)

## Out of scope

- Voice/text role description (multi-modal onboarding) — Phase 2 of Genesis Studio, separate program
- SOP-PDF / wiki / Slack uploads as Genesis input — Phase 2
- Name parser fixes (covered by `fix-onboarding-first-hire-name-parser-prefers-named-pattern`)
- Tenant-resolver mismatch (covered by `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`)
- Provider-key 500 (covered by `fix-onboarding-provider-key-500-friendly-error`)

## Effort

~250-400 LOC across the gateway handler + Genesis plumbing + 1 fallback branch + 2 tests + a small SPA banner. ~6-10 hours of focused work. **P0 — the runbook's Scene 3 talk-track ("Skycot turns that sentence into a real vertical") is unrecordable as currently written until Genesis actually runs at this surface.**
