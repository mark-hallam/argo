# fix-onboarding-genesis-manifest-memory-eval-persistence

**Owner**: codex.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-genesis-manifest-memory-eval-persistence`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/onboarding-hire-an-ai-teammate.md` F6 [P1].

🟠 **P1 — Even after PR #2400 wires Genesis on first-hire, the Genesis plan's `manifest.memory_schema` + `manifest.eval_seeds` + action vocabulary are computed by Genesis but dropped on the floor when assembling the HireDraft. The worker's memory namespace is created empty; no eval suite is registered. If the camera drills into `/workers/{id}/memory` or `/workers/{id}/evals` tabs, it finds empty surfaces.**

## Root cause

`crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:267-277` builds `HireDraft` from the Genesis plan but only extracts `charter` + `duties`. The full `plan.manifest` carries:
- `memory_schema` — the schema describing what the worker is allowed to remember
- `eval_seeds` — initial eval cases to register
- action vocabulary — the worker's specific toolset

None of these are persisted. The worker is created via `crates/argo-identity/src/store_hire.rs:199`:
```rust
MemoryNs::new(tenant_id, id, MemoryScope::WorkerSelf)
```
— empty namespace, no schema, no eval suite.

The synthesised charter mentions `Genesis plan id: <id>` (via `routes_onboarding_genesis.rs:140-143`), so the operator sees a plan id, but clicking through to /workers/{id}/memory or /workers/{id}/evals shows nothing.

Per the function-vs-dressing rule: this is **theatre the camera might see** if the demo drills into a memory or evals tab. The charter line "Genesis plan id: X" is a promise the rest of the page does not back up.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F6 [P1] — Genesis-synthesised memory schema + eval seeds are computed but never persisted

## Fix recipe

1. **Extend `HireDraft` to carry the synthesised data**:
   ```rust
   pub struct HireDraft {
       // ... existing fields ...
       pub memory_schema: Option<MemorySchema>,
       pub eval_seeds: Option<Vec<EvalSeed>>,
       pub action_vocabulary: Option<Vec<ActionToken>>,
       pub genesis_plan_id: Option<String>,
   }
   ```

2. **Pull the fields off the plan in `routes_onboarding::first_hire_response`**:
   ```rust
   draft.memory_schema = plan.manifest.memory_schema.clone();
   draft.eval_seeds = plan.manifest.eval_seeds.clone();
   draft.action_vocabulary = plan.manifest.action_vocabulary.clone();
   draft.genesis_plan_id = Some(plan.id.clone());
   ```

3. **Persist them in `IdentityStore::create_worker_from_draft`**:
   - Memory schema → call existing `MemoryNs::with_schema(tenant_id, id, scope, schema)` or add the method
   - Eval seeds → call `register_worker_eval_seeds(worker_id, &seeds)` (add this method if missing)
   - Action vocabulary → persist on the worker row's `tool_permissions` field

4. **Surface the plan provenance on `/workers/{id}/overview`** — add a "Plan provenance" chip next to the charter showing `Genesis plan: <plan_id>` clickable to a future modal. For v1.0 this can be a simple read-only chip; for v1.1 it opens a "see what Genesis chose" view.

5. **Add a test** at `crates/argo-cli/tests/onboarding_first_hire_persists_manifest.rs`:
   - POST first-hire
   - Assert the worker's memory namespace has a non-empty schema
   - Assert ≥1 eval seed registered for the worker
   - Assert the genesis_plan_id is queryable via `/api/v1/workers/{id}/overview`

## Acceptance criteria

- [ ] After first-hire, `argo memory ns show <worker_id>` shows the schema populated from the Genesis plan
- [ ] After first-hire, `argo eval list --worker <worker_id>` shows ≥1 eval seed
- [ ] `/api/v1/workers/{id}/overview` includes `genesis_plan_id`
- [ ] `/workers/{id}/overview` SPA renders a "Plan provenance" chip
- [ ] New cargo test passes
- [ ] Cold rerun of claim-3 LIVE test still passes

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` (extract plan manifest)
- `crates/argo-identity/src/store_hire.rs` (HireDraft schema + persistence)
- `crates/argo-genesis/src/templates.rs` (confirm manifest fields exist)
- `crates/argo-memory/src/namespace.rs` (with_schema constructor)
- `crates/argo-eval/src/registry.rs` (register_worker_eval_seeds)
- `web/components/skycot/worker/WorkerOverviewContent.tsx` (plan provenance chip)
- `crates/argo-cli/tests/onboarding_first_hire_persists_manifest.rs` (NEW)

## Out of scope

- Genesis invocation itself (covered by `fix-onboarding-first-hire-invoke-genesis-plan`)
- Voice/SOP-upload as Genesis input (Phase 2)
- Plan-provenance modal (v1.1 polish; v1.0 = chip only)

## Effort

~300-500 LOC across Rust + SPA + test. ~6-10 hours. P1 because once Genesis is invoked (P0 brief lands), the missing persistence becomes visible to anyone drilling into memory/eval tabs.
