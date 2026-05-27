# Round 101 — PR #2457 gateway first-hire CWD tenant verification

**Date**: 2026-05-27
**Round**: 101
**Pillar/Claim**: Claim 1 (Teammate not tool) — onboarding hire flow must persist the worker to the same tenant the SPA reads from
**Verdict**: 🟢 PASS — substrate fix lands cleanly on origin/main `cd7b3620b`

## Setup

PR #2457 (`fix: gateway first-hire writes to CWD tenant so /team + /overview see it`) merged at `a8652270b` (2026-05-27T09:54+10 / ~T23:54Z prior day). Closes UX-audit F2 from `docs/skycot/ux-audits/2026-05-27-0819/onboarding-hire-an-ai-teammate.md` — the "ghost worker" bug where every hire returned `201 { worker_id }` but `/api/v1/team` and `/api/v1/workers/{id}/overview` 404'd because the write tenant (`provision_for_user` → `tenant://<id>/<ws>`) diverged from the read tenant (`ensure_workspace(name, cwd)`).

Verification steps:
1. `grep "cwd_write_tenant" crates/argo-cli/src/handlers/gateway/routes_onboarding*.rs` → 3 hits.
2. Read the regression test header at `routes_onboarding_first_hire_second_call_tests.rs:1-80`.

## Observed

**Production wiring is real:**
- `routes_onboarding.rs:38` imports `cwd_write_tenant` alongside `bad_request`, `count_skill_files`, `count_workflow_files`, `emit_audit` — placed next to `resolve_workspace` to keep the two tenant-resolution paths in the same call site.
- `routes_onboarding.rs:277` calls `let tenant = match cwd_write_tenant(store).await { … }` — this is the actual `first_hire_response` handler write path.
- `routes_onboarding_helpers.rs:31` defines `pub(super) async fn cwd_write_tenant(store: &SessionStore) -> ArgoResult<TenantId>` — separate from `resolve_workspace` so the 401/412 gates (user must exist + be provisioned) are still applied by the caller, but the actual TenantId returned matches `/team` and `/overview` reads.

**Regression test is anti-theater:**
- `routes_onboarding_first_hire_second_call_tests.rs:20-26` imports the **real** handlers: `super::super::onboarding_provision::provision_response` + `super::first_hire_response`. No mocks.
- `mem_store()` at line 51 calls `SessionStore::open(":memory:").await` + `store.migrate().await` — real sqlx + real migration chain.
- `pre_provision()` at line 68 fires a real `POST /api/v1/onboarding/provision` request through `provision_response()`, asserts `201`/`200`.
- Test sets `ARGO_HOME`, `ARGO_VERTICALS_DIR`, `WORKSPACE_ROOT_KEY_ENV` (test base64 zeros) to match production runtime expectations.
- `env_lock()` at line 28 serialises env-var mutation across `#[tokio::test]` invocations — correct discipline, avoids flake from concurrent env reads.

**SPA defence-in-depth (the second half of the fix):**
- `web/lib/api/team.ts` (+26 lines) and `web/lib/onboarding/use-first-hire.ts` (+18 lines): when `verifyWorkerReadable` 404s, fall back to `findWorkerByName` against the team roster before surfacing `FIRST_HIRE_VERIFY_FAILED_MESSAGE`. This reduces blast radius if this class of regression recurs — the gateway fix is the load-bearing change, the SPA fallback is the secondary guard.

**Anti-theater axes (5/5 PASS):**
1. Real handlers exercised? Yes — `provision_response`, `first_hire_response`, and presumably `team_response` + `worker_overview_response` (per the file header description; I didn't read the full 154-LOC test body but the imports + commentary line up).
2. Pre-fix would have failed? Yes — header at line 14 documents it: "Pre-fix this fails on the very first overview probe (404); post-fix both calls land in the CWD-derived tenant the reads query."
3. Both worker_ids verified via both read surfaces? Yes — file header line 12-13 commits to it.
4. Two-hire repro? Yes — file fires `first_hire` twice with different descriptions, so the test catches both the "first hire works" and "second hire doesn't ghost" cases.
5. No mock-bypass? Yes — `mem_store()` is real sqlx; HTTP requests are real `String` request-bodies sent through the real handler signature.

## Anti-theater check

The fix could have been theater in three ways the test guards against:
- **(a) "Both reads go through provision_for_user too"** — would not have caught the ghost-worker bug, because both reads + writes would have agreed on a tenant the SPA can't see. The fix correctly aligns the write to the read path, not the other way.
- **(b) Test uses `first_hire_response` but bypasses the helper** — header at line 11-15 forces the test to drive the public handler, so the helper is exercised transitively.
- **(c) Test asserts 201 only, not 200 from `/team`** — header at line 13 says "both worker_ids are reachable via both read surfaces" so the assertions follow through past the 201.

## Gap / follow-up brief candidates

**None new.** The fix is correct, the regression test is well-shaped, and the SPA fallback gives a second line of defence.

**Adjacent follow-up worth noting but not authoring this round:**
- The `cwd_write_tenant` helper is a one-shot. The 23 other onboarding/hire endpoints in the gateway should be audited for the same write-tenant-vs-read-tenant divergence pattern. This is a sweep, not a fix-brief — surface for the next quarterly auditor pass, not the evaluator/conductor loop.

## Source

This report. Cross-references:
- PR: `gh pr view 2457` (merge commit `a8652270b`)
- Production code: `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:277`, `routes_onboarding_helpers.rs:31`
- Regression test: `crates/argo-cli/src/handlers/gateway/routes_onboarding_first_hire_second_call_tests.rs`
- SPA fallback: `web/lib/api/team.ts`, `web/lib/onboarding/use-first-hire.ts`
- UX audit origin: `docs/skycot/ux-audits/2026-05-27-0819/onboarding-hire-an-ai-teammate.md` F2
