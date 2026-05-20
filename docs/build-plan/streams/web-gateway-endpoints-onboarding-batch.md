# web-gateway-endpoints-onboarding-batch — implement 3 missing onboarding endpoints

**Owner**: claude (or codex).
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-gateway-endpoints-onboarding-batch`.
**Priority**: P1 — wave-3 onboarding audit found 3 of 4 onboarding endpoints unimplemented. The signup → onboarding path silently 404s as success today; users hard-stick on step 5 with no fallback.
**Sizing**: ~300-500 LOC (3 handlers + storage + tests).

## Why this brief exists

From the wave-3 onboarding audit (`docs/skycot/audit/onboarding/2026-05-20-wave-3-onboarding-e2e.md`):

> Three of four onboarding endpoints have no backend. Only
> `/api/v1/onboarding/provision` exists in `crates/`; `/provider-key`,
> `/install-vertical`, `/first-hire` are unimplemented. First-hire
> has no fallback so users hard-stick on step 5.

A new user reaches step 5 (first-hire), submits, and the page just
hangs because the gateway 404 is silently treated as success by
the hand-rolled fetch at `web/app/(setup)/onboarding/pick-vertical/page.tsx:74`.

## Endpoints to implement

1. `POST /api/v1/onboarding/provider-key` — body `{ provider: "anthropic" | "openai" | "google" | "azure" | ..., api_key: string }`. Stores encrypted key in workspace config. Returns `{ validated: true, masked_key, test_cost, test_tokens }`. **Contract-drift note (Wave-11 audit 2026-05-21):** the existing web client `web/lib/onboarding/use-provider-key.ts:20-24` expects `{ validated, test_cost, test_tokens }`. Match that shape. The server should perform a low-cost test call (e.g. 1-token completion) to validate the key + report cost/tokens of the validation call back to the UI. Also: when implementing the picker, source provider logos from https://svgl.app/ (operator directive — see `feedback_logos_from_svgl_app.md`) — Anthropic + OpenAI + Google + Azure at minimum.
2. `POST /api/v1/onboarding/install-vertical` — body `{ vertical_id: string }`. Installs vertical template + seeds workflows + skills for that vertical. Returns `{ installed: true, workflows_added: N, skills_added: M }`.
3. `POST /api/v1/onboarding/first-hire` — body `{ description: string, vertical_id: string }`. Spawns the first worker for the workspace. Returns `{ worker_id: string, name: string }`.

## Storage

- `workspace_provider_keys` table (additive) with encrypted-at-rest values
- `workspace_installed_verticals` table or column
- `workers` table (existing) — first-hire just inserts via existing CreateWorker flow

## Steps

1. Survey existing `/api/v1/onboarding/provision` to understand workspace bootstrap shape.
2. Decide encryption strategy for provider keys (use existing SQLCipher if available, else AES-GCM with a workspace key).
3. Implement install-vertical as: read vertical manifest (TOML) → bulk-insert workflow/skill rows under the workspace.
4. Implement first-hire to call into existing `CreateWorker` / `Worker::from_description`.
5. Each handler is principal-scoped; cross-tenant guard.
6. Wire into the route dispatch.

## may-touch

- New: `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs`
- New: tests
- `crates/argo-store/src/workspace_keys.rs` (or extend existing config storage)
- Migration if needed
- `crates/argo-cli/src/handlers/gateway/routes.rs`
- `web/app/(setup)/onboarding/pick-vertical/page.tsx` — fix the silent-404 bug at line 74 (use the proper `apiPost` client; surface errors)

## must-NOT-touch

- `/api/v1/onboarding/provision` (already shipped; don't break)
- Vertical-template definition format
- `argo-vertical-manifest` parser

## Acceptance criteria

- [ ] All 3 endpoints return 200 with realistic data; cross-tenant returns 403.
- [ ] Provider-key endpoint stores encrypted and returns masked key (`sk-...abc1`).
- [ ] Install-vertical reads the vertical manifest and seeds workflows + skills.
- [ ] First-hire creates a real worker row visible at `/workers/<new-id>`.
- [ ] Web /onboarding/first-hire success now actually redirects to `/workers/<new-id>` (no more 404 fall-through).
- [ ] `cargo clippy --workspace -- -D warnings` clean.
- [ ] At least 6 unit tests + 1 e2e integration smoke.

## References

- Audit: `docs/skycot/audit/onboarding/2026-05-20-wave-3-onboarding-e2e.md`
- Sibling: `web-onboarding-f1-wizard-shell` (client side)
