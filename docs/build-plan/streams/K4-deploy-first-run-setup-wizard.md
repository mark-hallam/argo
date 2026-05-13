# K4 — First-run setup wizard (workspace init + browser onboarding)

**Owner**: claude. **Depends on**: E0-frontend-foundation MERGE (need the AppShell route group + typed gateway client); at least one J* endpoint MERGED so the wizard talks to a real gateway. **BLOCKED on E0 MERGE; flip to TODO when E0 lands.**
**Authored against**: origin/main `08c4ff04` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh K4-deploy-first-run-setup-wizard`.

**Drift-fix amendment 2026-05-13** (vs HEAD 6159b789): brief-cleanup sweep updated HTTP-route surface. (1) `crates/argo-gateway/src/router.rs` is a chat-adapter routing module (resolves `TenantId` / `AgentName` / `SessionId` from Slack/Discord/Telegram `InboundMessage`), NOT an HTTP-route registrar. (2) Similarly `crates/argo-gateway/src/adapters/{slack,discord,telegram}.rs` are chat platform adapters, not HTTP handlers — adding `adapters/setup.rs` there is a category mismatch. (3) **Actual HTTP route dispatcher**: lives in `crates/argo-cli/src/handlers/gateway/routes*.rs`. Registering `GET/POST /api/v1/setup` + `POST /api/v1/setup/complete` requires editing: `crates/argo-cli/src/handlers/gateway/routes.rs` (dispatch), `crates/argo-cli/src/handlers/gateway/routes_auth.rs` (is_public_get allowlist for unauth'd first-run), `crates/argo-cli/src/handlers/gateway/mod.rs` (mod decl), `crates/argo-cli/src/handlers/gateway/routes_setup.rs` (NEW, ≤250 LOC). Drop `crates/argo-gateway/` from may-touch unless the brief intentionally publishes shared DTOs there (in which case keep one `dto.rs` for serde types only).

> SOURCE: DEPLOYMENT-PATH DIRECTIVE v3 §3 (Tier 5b). **Tier K (deployment recipe).** Bridges the gap between "stack is running" (K1/K2/K3 land you here) and "operator can actually use it." First boot detects an unconfigured workspace, redirects browser visits to `/setup`, walks the operator through workspace dir / sqlite path / default tenant / root bearer token (generated) / provider key prompts. CLI parity via `argo setup`.

## Scope

EXTEND `crates/argo-cli/src/handlers/setup.rs` (≤ 60 LOC delta — file EXISTS, verified 2026-05-12):

- Add `pub struct SetupState { workspace: PathBuf, db_path: PathBuf, default_tenant: TenantId, bearer_token: String, provider_keys: HashMap<String, String>, completed: bool }`
- Add `pub fn is_first_run(workspace: &Path) -> bool` — returns true if `<workspace>/.argo/setup.complete` marker missing.
- Add `pub fn mark_complete(workspace: &Path) -> ArgoResult<()>` — writes marker.

EXTEND `crates/argo-cli/src/handlers/setup_steps.rs` (≤ 80 LOC delta — file EXISTS) — wire `SetupState` into existing step orchestrator. Existing tests in `tests/setup_wizard.rs` already cover step plumbing; this brief integrates the marker check.

NEW `crates/argo-gateway/src/adapters/setup.rs` (NEW, ≤ 180 LOC) — gateway adapter for the wizard:

- `pub async fn get_setup_status() -> SetupStatus { first_run: bool, completed_steps: Vec<String>, pending_steps: Vec<String> }` — endpoint `GET /api/v1/setup`
- `pub async fn submit_setup_step(step: SetupStep) -> SetupStepResult` — endpoint `POST /api/v1/setup` (one step at a time; idempotent per-step)
- `pub async fn complete_setup() -> SetupComplete { bearer_token: String }` — endpoint `POST /api/v1/setup/complete` (returns the generated bearer token ONCE; subsequent calls 410 Gone).

EXTEND `crates/argo-gateway/src/router.rs` (≤ 15 LOC delta — file EXISTS; this is the ROUTER, not handlers/setup.rs which doesn't exist) — register the 3 setup routes. The /setup endpoints are EXEMPT from bearer-auth middleware (pre-setup the operator has no token).

NEW `web/app/(setup)/setup/page.tsx` (NEW, ≤ 220 LOC) — multi-step wizard UI:

- Step 1: workspace directory (read-only display of detected path)
- Step 2: default tenant name (text input; default `default`)
- Step 3: provider keys (text inputs for `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`; all optional; mask after save)
- Step 4: review + complete; clicking "Complete setup" calls `POST /setup/complete` and displays the generated bearer token with one-click copy
- After completion: redirect to `/` with the bearer token persisted to localStorage (operator copies it to their auth-bypass env for non-browser CLI use)
- Uses E0's `<AppShell>` + typed gateway client

NEW `web/lib/api/setup.ts` (NEW, ≤ 90 LOC) — typed client:

- `getSetupStatus() -> Promise<SetupStatus>`
- `submitSetupStep(step: SetupStep) -> Promise<SetupStepResult>`
- `completeSetup() -> Promise<SetupComplete>`
- Uses E0's `<ApiError>` envelope.

EXTEND `web/app/layout.tsx` (≤ 15 LOC delta) — root layout adds a middleware-style check: on first render, fetch `/api/v1/setup`; if `first_run: true` and current path is not `/setup`, redirect to `/setup`.

NEW `web/app/(setup)/setup/page_tests.tsx` (NEW, ≤ 100 LOC) — Playwright or Vitest+React Testing Library, depending on what E0 selects. 4 tests: shows-wizard-on-first-run, persists-step-state-between-reloads, completes-and-returns-bearer-token, post-complete-redirects-to-root.

### CLI parity

EXTEND `crates/argo-cli/src/handlers/setup.rs` (already covered above; existing `argo setup` CLI is wired through). Acceptance: `argo setup --non-interactive --tenant default --token <auto>` completes setup without browser.

### Migrations

The argo binary already runs migrations on first DB connect. No new migration logic needed in this brief. The wizard's "step 1: workspace directory" tells argo where to put the sqlite DB; migrations then run on the next argo-binary action.

This is NOT in scope: multi-tenant setup at first run (single tenant in v1; operator adds tenants via `argo tenant create` after wizard); OIDC / SAML configuration during setup (operator wires post-setup); MCP-server registration during setup (post-setup via `argo connector add`); auto-detection of cloud-provider env (AWS / GCP creds — operator configures via env post-setup); workspace-import / restore-from-backup (separate brief); migration-from-previous-version flow (operator runs `argo migrate run` between versions); generating SSH keys / TLS certs (out of scope; TLS is K3's certbot path).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/setup.rs` (EXTEND, ≤ 60 LOC delta)
- `crates/argo-cli/src/handlers/setup_steps.rs` (EXTEND, ≤ 80 LOC delta)
- `crates/argo-cli/src/handlers/gateway/routes_setup.rs` (NEW, ≤ 250 LOC — per Drift-fix 2026-05-13: this is the actual HTTP-route home; `argo-gateway/src/router.rs` is a chat-adapter, NOT an HTTP router)
- `crates/argo-cli/src/handlers/gateway/routes.rs` (EXTEND, ≤ 8 LOC — register `/api/v1/setup` dispatch)
- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` (EXTEND, ≤ 5 LOC — add setup routes to `is_public_get` allowlist for pre-setup unauthed access)
- `crates/argo-cli/src/handlers/gateway/mod.rs` (EXTEND, ≤ 2 LOC — `pub mod routes_setup;`)
- `web/app/(setup)/setup/page.tsx` (NEW, ≤ 220 LOC)
- `web/app/(setup)/setup/page_tests.tsx` (NEW, ≤ 100 LOC)
- `web/lib/api/setup.ts` (NEW, ≤ 90 LOC)
- `web/app/layout.tsx` (EXTEND, ≤ 15 LOC delta — first-run redirect)

## Files you must NOT touch

- `crates/argo-cli/src/handlers/setup_render.rs` (TUI/CLI render path — out of scope; CLI parity uses existing handler)
- `crates/argo-cli/tests/setup_wizard.rs` + `setup_check.rs` (existing tests; this brief should EXTEND with new tests, not rewrite)
- `crates/argo-store/migrations/**` (no migration)

## Acceptance criteria

- [ ] `cargo build -p argo-cli -p argo-gateway` succeeds
- [ ] `cargo test -p argo-cli setup` passes (existing + new tests)
- [ ] Fresh workspace + browser → wizard renders at `/setup`
- [ ] Submitting wizard steps idempotent (re-submission of same step returns 200, no duplicate state)
- [ ] `POST /setup/complete` returns bearer token ONCE; subsequent calls return 410 Gone
- [ ] `/setup` route is exempt from gateway bearer-auth (verified)
- [ ] After completion: `<workspace>/.argo/setup.complete` marker exists; subsequent boots skip wizard
- [ ] `argo setup --non-interactive --tenant default --token-out <path>` writes generated token to path
- [ ] Path verification confirmed at claim time (router.rs exists; adapters/setup.rs new; web route group new)
- [ ] All new files < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-K4-deploy-first-run-setup-wizard`
- PR title: `deploy: first-run setup wizard (CLI + browser; bearer-token render-once) (K4)`

## Notes for the implementer

- This brief is BLOCKED on E0-frontend-foundation MERGE because:
  - It uses `<AppShell>` route shell from E0
  - It uses E0's typed gateway client
  - It uses E0's `<ApiError>` envelope
  - The `(setup)` route group convention matches E0's `(app)` / `(hire)` pattern
  Conductor flips this row TODO when E0 MERGES.
- The /setup endpoints MUST be auth-bypassed at the gateway middleware level — pre-setup the operator has no bearer token. The bearer-auth middleware should check for a setup-completion marker and skip auth when the marker is absent. Verify the auth-middleware location at claim time (likely `crates/argo-cli/src/handlers/gateway/auth_middleware.rs` or similar).
- The bearer-token render-once pattern is load-bearing for security. Storing it server-side anywhere risks accidental disclosure via debug endpoints. Generate it via `openssl rand -hex 32` equivalent (use `rand` crate's OS-level entropy), display once to operator, persist the HASHED form server-side for auth comparison.
- localStorage persistence on the web side is convenience for the operator's first session. The real source of truth is the bearer token they copy to env/keyring/etc. Document this in the wizard's final-step copy.
- The wizard's step-state persistence (step 2 reload doesn't lose step 1's input) lives client-side; gateway is stateless per-step. Use React state + URL query params + localStorage backup.
- DO NOT auto-generate provider API keys. Those are external services; operator pastes existing keys (or skips if not yet configured).
- DO NOT email / SMS the generated token. It's render-once in the browser; operator owns the copy.
- The `(setup)` route group convention isolates the wizard from the main app shell — different chrome (no nav, no auth nag banner, no organization switcher). Mirror E0's route-group structure.
- For the migration concern: argo's binary handles migrations on first connect. The wizard's "workspace directory" step writes a stub argo.toml; the next argo-cli or argo-gateway action runs migrations. The wizard does NOT trigger migrations directly — that's a separate concern.
- Test fixtures: `cargo test -p argo-cli setup_wizard` already exists and tests step plumbing in isolation. New tests in this brief cover the marker-file lifecycle + the gateway adapter (mock-mode).
- E0 lands the AppShell with the auth-bypass-banner already wired. K4 just routes through it for the wizard surface.
- Coordinate with `mission-system-keyring-integration` (in-flight): if that brief lands first, the wizard's bearer-token render-step can offer "save to OS keyring" as an option. If not, render-once + operator-copy is sufficient.
