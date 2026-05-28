# rebase-fix-onboarding-genesis-manifest-memory-eval-persistence

**Authored against:** origin/main `a5df90c41` on 2026-05-29 by claude deep-triage-agent-3.
**Validate before claim:** `bash scripts/check-brief-freshness.sh rebase-fix-onboarding-genesis-manifest-memory-eval-persistence`

## What

Re-implement the original brief `streams/fix-onboarding-genesis-manifest-memory-eval-persistence.md` against current `origin/main` HEAD.

The original PR #2606 (claude, 531 LOC) was authored against an older `origin/main` and conflicts on:

- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` — 1 merged PR since (#2524 expanded install-vertical receipt with memory + gates flags). File is 15490 bytes / under 500 LOC on main.
- `web/components/skycot/worker/WorkerOverviewContent.tsx` — 2 merged PRs since touching the overview shell.
- New files the PR adds (`routes_onboarding_genesis_seed.rs` + `routes_onboarding_first_hire_persists_manifest_tests.rs`) do NOT exist on main — net-additive, no rebase needed for the new files themselves.

Current shape on main (verify with `cat` before authoring):

- `routes_onboarding.rs` has a `memory_schema_attached` boolean flag at line 198 + line 227+239 (from #2524) — this signals whether install-vertical wired a schema, but is NOT the same as actually seeding 4 `MemoryFile::Memory` rows into the new worker's namespace.
- `MemoryStore::append_in_namespace` is the canonical write path (used in `routes_retrain_helpers.rs:227`, `routes_worker_memory_helpers.rs:279`, `routes_worker_memory.rs:107`). The original PR's approach of calling it post-`create_worker_from_draft` is the correct pattern.
- `first_hire_response` is the call-site to instrument with the new namespace-write.
- `/workers/<id>/memory` endpoint (`routes_worker_memory.rs`) already reads from the namespace store — no SPA API change needed for the chip.
- `WorkerOverviewContent.tsx` has had 2 commits since — the small Genesis chip render needs to be re-applied carefully (verify no other overview chips clash with the same data-testid).

The bug (Genesis-computed `memory` / `evaluation` / `actions` payloads fall on the floor at first-hire — F6 [P1] in `docs/skycot/ux-audits/2026-05-26-2007/onboarding-hire-an-ai-teammate.md`) is still load-bearing on main: no `MemoryStore::append_in_namespace` call exists in the first-hire path.

## Acceptance

Same as the original brief:

1. `routes_onboarding::first_hire_response` follows `create_worker_from_draft` with a best-effort `MemoryStore::append_in_namespace` call that seeds 4 `MemoryFile::Memory` rows in the new worker's namespace:
   - row 1: Genesis plan provenance (plan id + version + timestamp)
   - row 2: Genesis memory schema (the operator's `memory_schema` JSON)
   - row 3: Genesis eval seeds (the operator's `evaluation` payload)
   - row 4: Genesis action vocabulary (the operator's `actions` payload)
2. Best-effort: a write failure does NOT block the first-hire response (log the failure, return 200 with worker shell). The SPA can fall back to the raw plan if the namespace read fails later.
3. `WorkerOverviewContent.tsx` grows a small "Genesis seeded" chip (presence flag), extracted from the synthesised charter (no new `/overview` API field).
4. New persists-manifest test file exercises the 4-row write end-to-end against a tmp tenant.
5. New `routes_onboarding_genesis_seed.rs` extraction module if `routes_onboarding.rs` would breach 500 LOC after the additions; otherwise inline.

## Sensible deviations

- Preserve #2524 install-vertical receipt expansion (`memory_schema_attached` + `approval_gates_added`). The new seed call is layered AFTER that, not replacing it.
- Verify `routes_onboarding.rs` post-additions stays under 500 LOC; if not, extract the seed helper into `routes_onboarding_genesis_seed.rs` (the original PR already planned this — preserve the extraction).
- The `WorkerOverviewContent.tsx` chip render must not collide with any new chips added by the 2 intervening commits — verify data-testid uniqueness.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs`
- new `crates/argo-cli/src/handlers/gateway/routes_onboarding_genesis_seed.rs`
- new `crates/argo-cli/src/handlers/gateway/routes_onboarding_first_hire_persists_manifest_tests.rs`
- `crates/argo-cli/src/handlers/gateway/mod.rs` (register the new test module)
- `web/components/skycot/worker/WorkerOverviewContent.tsx`

## Verify

- `cargo test -p argo-cli routes_onboarding_first_hire_persists_manifest_tests`
- `cargo clippy -p argo-cli --tests`
- `pnpm --dir web exec vitest run components/skycot/worker`
- `pnpm --dir web typecheck`
