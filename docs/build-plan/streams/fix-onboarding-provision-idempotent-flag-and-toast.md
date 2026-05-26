# fix-onboarding-provision-idempotent-flag-and-toast

**Owner**: codex.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-provision-idempotent-flag-and-toast`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/onboarding-hire-an-ai-teammate.md` F10 [P2].

🟡 **P2 — Provision endpoint logs `idempotent:true` on first call AND on re-call. No signal in the response distinguishes "I just provisioned your fresh workspace" from "you already had one." Wizard misses a celebration moment.**

## Root cause

`crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs:38-66` — `resolve_workspace` calls `provision_for_user` which returns `outcome.created`, but the workspace step's API response says `idempotent:true` regardless.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F10 [P2] — provision logs idempotent silent on first call

## Fix recipe

1. **Surface `created:true|false` in the provision response payload**. Adapt `routes_onboarding_helpers.rs` to return `{idempotent:bool, created:bool}`.
2. **In `web/app/(setup)/onboarding/workspace/page.tsx`**, on response with `created:true` show a confirmation toast "Workspace created — welcome to Skycot."
3. **Test**: integration test asserts the field is present in the response.

## Acceptance criteria

- [ ] Provision response includes `created` boolean.
- [ ] On `created:true`, wizard shows celebration toast.
- [ ] On `created:false`, current silent behavior preserved.
- [ ] Integration test covers both paths.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_onboarding_helpers.rs` (response field)
- `crates/argo-cli/src/handlers/gateway/routes_onboarding.rs` (caller)
- `web/app/(setup)/onboarding/workspace/page.tsx` (toast)
- `crates/argo-cli/tests/onboarding_provision_created_flag.rs` (NEW)

## Out of scope

- Workspace-rename UI (post-MVP)
- Provision-failure recovery (separate concern)

## Effort

~40 LOC + 1 test. ~45min.
