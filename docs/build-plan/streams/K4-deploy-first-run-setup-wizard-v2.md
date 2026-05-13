# K4-deploy-first-run-setup-wizard-v2 — browser surface over existing `wizard_onboard.rs` 5-step plumbing

**Owner**: claude. **Depends on**: shipped `crates/argo-cli/src/handlers/wizard_onboard.rs` (`mission-operator-onboarding-wizard`).
**Authored against**: origin/main `a348e5d6` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh K4-deploy-first-run-setup-wizard-v2`.

> **REWRITE 2026-05-13 by conductor (CEO-authorized).** K4 V1 would have shipped a 3rd parallel onboarding wizard alongside the already-shipped `argo wizard onboard` (5-step, `wizard_onboard.rs`) and `argo onboard` (7-step, `onboard.rs`). V1 incorrectly framed `argo setup` (the provider-key probe) as the CLI parity, and authored `SetupState` + `is_first_run` + `mark_complete` from scratch instead of consulting the existing first-run detection. V2 ships **only the browser surface** as an HTTP adapter over the already-shipped 5-step plumbing.

## Scope

`wizard_onboard.rs` already has the right plumbing — `run_steps<P: PromptIo, V: KeyVerifier>` at line 148 takes a generic `PromptIo` for I/O and `KeyVerifier` for the provider-key check. The CLI ships a stdin/stdout `PromptIo`; the browser surface ships an HTTP-buffered `PromptIo`.

1. **NEW `crates/argo-cli/src/handlers/wizard_onboard_http.rs`** (≤200 LOC) — an HTTP-driven `PromptIo` impl that holds an `Arc<Mutex<WizardSession>>` per-tenant where each step's prompt arrives at the browser, the browser POSTs an answer, and the next step is computed. State machine: `Idle → AwaitingProviderKey → AwaitingOperator → AwaitingVertical → AwaitingWorker → Complete`. Reuses the existing `run_steps` for the step logic; this module only carries the HTTP-state plumbing.

2. **3 new gateway routes** in `crates/argo-cli/src/handlers/gateway/routes_wizard.rs` (NEW, ≤150 LOC):
   - `GET /api/v1/wizard` — returns current step + question prompt (JSON).
   - `POST /api/v1/wizard/answer` — submits answer for current step, advances. Returns next step.
   - `POST /api/v1/wizard/reset` — operator can restart mid-wizard.

3. **NEW `web/app/(setup)/setup/page.tsx`** + supporting components (≤300 LOC) — Next.js page that hits the 3 endpoints, renders the appropriate input per step. Mirrors the CLI's prompts verbatim (don't reword — operators expect parity).

4. **First-run detection unified** — instead of authoring `is_first_run` from scratch, surface the existing `home::existing_operator_block` check (line 206 in wizard_onboard.rs) as a public helper that BOTH the CLI and the new browser route consult. Single source of truth.

5. **No new `SetupState` / `mark_complete`** — the wizard's completion is already signaled by `argo.toml` having a valid `[operator]` block + `argo.toml` `[providers.<id>]` block. The browser's "are we done?" check reuses that detection.

**Not in scope**: changing the 5-step CLI flow; adding a 6th step; building a third wizard; per-tenant routing of wizards (v1 is single-operator).

## Files you may touch

- `<NEW: crates/argo-cli/src/handlers/wizard_onboard_http.rs>` (≤200 LOC) — HTTP-driven `PromptIo` impl + `WizardSession` state.
- `<NEW: crates/argo-cli/src/handlers/wizard_onboard_http_tests.rs>` (≤200 LOC) — 5 unit tests + 1 integration smoke.
- `<NEW: crates/argo-cli/src/handlers/gateway/routes_wizard.rs>` (≤150 LOC) — 3 route handlers.
- `crates/argo-cli/src/handlers/wizard_onboard.rs` — narrowly: export `existing_operator_block` as `pub` so the browser route can call the same detection. ≤5 modified lines.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs` + `routes_dispatch_post.rs` — narrowly: route `/api/v1/wizard*` to the new handler. ≤20 modified lines.
- `crates/argo-cli/src/handlers/gateway/routes_auth.rs::is_public_get` — narrowly: allow `/api/v1/wizard*` until operator is set up (first-run is the only time before auth). ≤3 modified lines.
- `<NEW: web/app/(setup)/setup/page.tsx>` + ≤2 supporting components — ≤300 LOC total.
- `<NEW: web/app/(setup)/setup/setup.test.tsx>` — minimum vitest coverage.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/wizard_onboard.rs`'s step functions (`step_detect`, `step_provider_key`, `step_operator`, `step_vertical`, `step_worker`) — these are the locked 5-step plumbing. The browser surface drives them via `PromptIo`; it does not re-author them.
- `crates/argo-cli/src/handlers/onboard.rs` — the 7-step `argo onboard` is a separate flow; do NOT consolidate this brief either replaces or competes with it.
- `crates/argo-cli/src/handlers/setup.rs` — provider-key probe; do NOT extend with onboarding state.
- `crates/argo-config/**` — `argo.toml` shape is the canonical operator-config; do NOT add a new file/state for "is first-run".

## Acceptance criteria

- [ ] `argo wizard onboard` CLI behavior **unchanged** — pre-existing tests pass.
- [ ] `wizard_onboard_http::PromptIo` impl drives the same 5 steps as CLI via HTTP request/response cycle.
- [ ] 3 new routes wired: `GET /api/v1/wizard`, `POST /api/v1/wizard/answer`, `POST /api/v1/wizard/reset`.
- [ ] First-run detection: the same `existing_operator_block` check is used by both CLI and browser. No new `SetupState` struct.
- [ ] Browser surface walks through all 5 steps end-to-end against a wiremocked provider-key verifier.
- [ ] Once operator is set up (`argo.toml [operator]` present), `/api/v1/wizard` returns `Complete` and the page redirects to the dashboard.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `pnpm test` + `pnpm typecheck` clean in `web/`.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-K4-deploy-first-run-setup-wizard-v2`
- PR title: `gateway+web: browser surface over existing wizard_onboard 5-step plumbing (K4-deploy-first-run-setup-wizard-v2)`

## Notes for the implementer

- The load-bearing idea: **the CLI and browser are different I/O layers over the SAME 5-step flow.** Don't fork the flow. Don't author a parallel step list.
- `PromptIo` is the right abstraction — see how the CLI ships its `StdinPromptIo` impl. The browser impl is exactly the same shape but backed by an `Arc<Mutex<WizardSession>>`.
- The `KeyVerifier` trait is also generic — the browser path can pass the same `RealKeyVerifier` the CLI uses, OR a stubbed one for tests.
- This is the operator-visible v1 onboarding browser surface. Once landed, the K4 epic is complete (`docs/build-plan/PRODUCT_ROADMAP.md` Track ?).
- ~650 LOC total (vs K4-v1's ~1000+ across SetupState + 3 routes + page + tests). Reusing existing plumbing keeps the diff small and avoids the third-parallel-wizard antipattern CLAUDE.md warns against.
