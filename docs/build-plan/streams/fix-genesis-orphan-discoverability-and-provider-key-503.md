# fix-genesis-orphan-discoverability-and-provider-key-503

**Owner**: claude.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-genesis-orphan-discoverability-and-provider-key-503`.
**Source**: ux-e2e-audit-T2309MEL `onboarding-hire-an-ai-teammate.md` F1 [P0].

🔴 **P0 DEMO-GATE — Genesis is the operator-locked investor-moment ("the AI Teammate Onboarding Studio that distinguishes Skycot from 'ChatGPT with prompts'" — memory `feedback_genesis_is_ai_teammate_onboarding_studio`) but `/genesis` is an orphan page: zero inbound links from nav, landing CTA, footer, or any wizard. Even if reached, `POST /api/v1/genesis` returns 503 `genesis_provider_key_missing` on the dev gateway. The investor walkthrough cannot be recorded.**

## Root cause

Two coupled defects gate the Genesis investor moment:

1. **No discoverable entry point.** `rg 'href="/genesis|router.push\("/genesis"' web/` returns zero matches. The canonical nav at `web/components/skycot/nav-items.tsx:22-30` lists Approvals / Team / Workers / Cohorts / Skills / Workflows / Audit — Genesis is not present. Marketing footer + `/docs` landing + `/onboarding/welcome` ("Get started" button at `web/app/(setup)/onboarding/welcome/page.tsx:63`) all route past `/genesis`.

2. **Gateway endpoint 503.** `curl -X POST http://127.0.0.1:7400/api/v1/genesis -H 'Content-Type: application/json' -d '{}'` returns:
   ```json
   {"error":{"code":"genesis_provider_key_missing","message":"Set ARGO_GENESIS_PROVIDER_KEY to enable POST /api/v1/genesis (LLM-driver cost guardrail)"}}
   ```
   The streaming pipeline `useGenesisWorkerDraft` (`web/lib/api/genesis.ts`) would surface "Couldn't reach the Genesis stream" on every dev attempt.

The orphan-page state is what audits its own to-be-recorded narrative. The investor cannot click a button to reach the page that proves the most distinctive product claim.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F1 [P0] — Genesis orphaned + gateway 503

## Fix recipe

Operator-decision first (which surface is canonical), then ship one of:

1. **Option A (preferred per audit) — Make `/genesis` reachable from the canonical wizard.**
   - Replace `/onboarding/welcome`'s "Get started" target. Currently routes to `/onboarding/workspace`; change to `/genesis` for first-hire flow.
   - Add `/genesis` to the in-app top-nav `NAV_ITEMS` at `web/components/skycot/nav-items.tsx:22-30` between Workers and Cohorts.
   - Wire the marketing `/landing` "Hire your first teammate" hero CTA at `web/app/landing/page.tsx` to `/genesis` (or to `/onboarding/welcome` if the welcome→genesis chain is kept).

2. **Option B — Promote the orphaned page into the wizard chain.**
   - Insert `/genesis` as step 2 of `/onboarding/*` (between workspace-name and connect-provider).
   - Update `OnboardingShell` step pip count to 6.

3. **Provider-key bootstrap** (independent of A/B):
   - Add `ARGO_GENESIS_PROVIDER_KEY` to `.claude/launch.json` env block alongside `ANTHROPIC_API_KEY` (or alias from it, since the operator already has Anthropic Pro access on the dev box).
   - Gateway code path lives at `crates/argo-cli/src/handlers/gateway/routes_genesis.rs` (verify exact filename via `rg 'genesis_provider_key_missing'`). Either alias the env var to `ANTHROPIC_API_KEY` so dev-mode bypasses the explicit guardrail, OR document in the runbook that the operator must `export ARGO_GENESIS_PROVIDER_KEY=<key>` before `scripts/restart-live-gateway.sh`.
   - Add a curl-smoke step to `scripts/demo-dry-run.sh` (or equivalent): `curl -fsS -X POST http://127.0.0.1:7400/api/v1/genesis -d '{}' | jq` should NOT return `genesis_provider_key_missing`.

4. **Cross-link from welcome pillar copy.** The pillar at `web/app/(setup)/onboarding/welcome/page.tsx:43-49` (data-testid `onboarding-pillar-genesis-studio`) already describes Genesis behavior. Make the pillar tile clickable, routing to `/genesis`. This is partial — see also existing brief `fix-onboarding-welcome-step-genesis-studio-framing` which authors the copy itself.

## Acceptance criteria

- [ ] At least one inbound link from a nav or landing surface lands the user on `/genesis` without typing the URL
- [ ] `POST /api/v1/genesis` returns 200 (or 4xx with payload validation, not 503) on the dev gateway after `scripts/restart-live-gateway.sh`
- [ ] `scripts/demo-dry-run.sh` (or new) asserts the Genesis endpoint is reachable before declaring recording-ready
- [ ] `pnpm --dir web exec vitest run components/skycot/nav-items` passes
- [ ] e2e walk: clean session → click whatever the chosen entry is → land on `/genesis` → stream begins (or surfaces a non-503 error)

## May-touch

- `web/components/skycot/nav-items.tsx` (~5 LOC for nav row)
- `web/app/(setup)/onboarding/welcome/page.tsx` (~10 LOC for pillar click target or CTA target)
- `web/app/landing/page.tsx` (~3 LOC for CTA target — if Option A)
- `.claude/launch.json` (~1 LOC for env var)
- `scripts/restart-live-gateway.sh` + `scripts/demo-dry-run.sh` (~10 LOC for boot env + smoke)
- `crates/argo-cli/src/handlers/gateway/routes_genesis.rs` (~5 LOC for env alias — verify file)

## Out of scope

- Genesis multi-modal upload UI (voice / SOP-PDF / wiki / Slack — separate Phase-2 deliverable per operator memory)
- The OTHER parallel wizard surface at `/hire/*` — covered by `fix-onboarding-hire-parallel-wizards-unify-or-document`
- Genesis manifest/memory persistence — covered by `fix-onboarding-genesis-manifest-memory-eval-persistence`

## Effort

S–M. ~30 LOC SPA + ~5 LOC gateway env + ~10 LOC script. ~45 min. P0 because the operator-named investor moment is non-recordable.
