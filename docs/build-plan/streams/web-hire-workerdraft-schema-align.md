# web-hire-workerdraft-schema-align — fix the SPA↔gateway WorkerDraft schema mismatch that collapses 5-teammate hire to "Mariah" clones

**Owner**: claude.
**Authored against**: origin/main `2fa4baf1` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-hire-workerdraft-schema-align`.
**Priority**: **P0 — wave-15 buyer-trial blocker #3.** Every 5-teammate hire falls back to `STUB_QUESTIONS` and produces 5 identical "Mariah"-style worker drafts. The fix unblocks the first-30-minutes user journey.

## Why this brief exists

Read `docs/skycot/audit/five-teammate-hire/` + the wave-15 final verdict at `docs/skycot/audit/go-no-go-journey-final/2026-05-21-wave-15-journey-final.md` (gap #4). Audit summary:

> "5-teammate hire — backend↔frontend `WorkerDraft` schema mismatch makes live-gateway path fall to stub, every hire collapses to 'Mariah'. No idempotency key, no name-uniqueness check, genesis-stub floods tenants with 'Mariah's."

The chain of failure:

1. SPA hire-flow Step 3 (Drafting) calls the gateway Genesis endpoint to fetch question sets specialized to the user's input.
2. Gateway returns a `WorkerDraft` JSON envelope whose schema has drifted from the SPA's TypeScript `WorkerDraft` interface in `web/lib/hire/draft-types.ts`.
3. JSON parse / type-check fails silently → SPA falls back to `STUB_QUESTIONS` from `web/lib/hire/genesis-stub.ts`.
4. `STUB_QUESTIONS` are intentionally generic ("Should they reply on your behalf, or draft replies for you to send?") and identical for every worker.
5. All 5 workers in a batch hire are stamped with the same generic answers → same persona-generation seed → 5 "Mariah" clones.
6. The user sees no error — the stub is transparent. They think their input was ignored.

This brief lands three things together:

1. **Schema alignment** — make `WorkerDraft` SPA-side match the gateway-side Rust struct byte-for-byte. Adopt a single source of truth (recommended: generate the TS from the Rust via `argo gateway dump-types --target=ts` if that command exists, or hand-mirror with a structural test).
2. **Loud failure** — when the gateway call fails for any reason (network, schema mismatch, 5xx), the SPA shows the user an explicit "couldn't reach Genesis — generic questions only" warning banner WITH a Retry button. No more silent stub fallback.
3. **Per-hire idempotency** — each hire submit carries a client-generated `idempotency_key` UUID v7. Gateway de-dupes on it within a 1-hour window. Prevents duplicate-hire storms when the user re-clicks Submit.

## Approach (concrete)

### Phase 0 — Capture the actual mismatch

1. **Audit the schema delta**:
   ```bash
   # Gateway-side Rust struct
   rg -n 'pub struct.*WorkerDraft|fields.*workerDraft' crates/argo-cli/src/handlers/gateway/ crates/argo-genesis/src/ crates/argo-types/src/
   # SPA-side TS interface
   cat web/lib/hire/draft-types.ts | grep -A 30 'interface WorkerDraft'
   ```
   Capture every field name + type. The audit's specific finding was that fields differ; the brief author MUST enumerate them all before authoring the fix (a one-line `cargo expand` + the existing TS interface is enough).

2. **Reproduce the bug locally**: run the SPA against the gateway, open the Network tab in /hire/step/3, watch the response → assert the parse failure with a console.error. This proves the bug exists in the current code (the audit was a few hours ago — verify it's still present after recent merges).

### Phase 1 — Schema alignment

3. **Decide the source of truth.** Two options:
   - **(a) Hand-mirror with a structural test**: write a vitest in `web/lib/hire/__tests__/workerdraft-schema-shape.test.ts` that asserts every gateway-side field is present in the TS interface (and vice versa). Failing test forces engineers to update both sides when schema drifts.
   - **(b) Generate from Rust**: if a generator exists (look at `argo gateway dump-types` or `cargo run -- gateway schema-export`), use it. Otherwise, option (a) is the v1 fix.

   Recommend option (a) for v1 — concrete, testable, no new tooling to maintain.

4. **Update `web/lib/hire/draft-types.ts`** to add/rename/remove fields so the TS interface matches the gateway's Rust struct. Keep the existing `version: 1` migration safety; bump to `version: 2` if any incompatible shape changes hit and the loader needs to invalidate v1 drafts.

5. **Update `web/lib/api/workers.ts` / `web/lib/hire/use-genesis-worker-draft.ts`** to handle the gateway's actual response shape. If field names use snake_case server-side (the gateway convention is snake_case in JSON payloads per #1907's pattern), do the camel↔snake mapping at the boundary.

### Phase 2 — Loud failure (no silent stub fallback)

6. **Add a fallback-mode banner** in `web/app/(hire)/hire/step/3/page.tsx` (or wherever Step 3 lives — verify with `rg -ln 'STUB_QUESTIONS' web/app/`):

   ```tsx
   {isUsingStub && (
     <Alert variant="warning" data-testid="genesis-fallback-banner">
       <AlertTitle>Generic questions only</AlertTitle>
       <AlertDescription>
         We couldn't reach Genesis right now, so we're showing generic
         questions. The worker we draft from your answers will be
         generic too. <button onClick={retry}>Retry</button>
       </AlertDescription>
     </Alert>
   )}
   ```

7. **Make `isUsingStub` observable** — `use-genesis-worker-draft.ts` returns a `{ source: 'gateway' | 'stub', error?: Error }` field. The hire flow's UI gates the banner on this signal.

8. **Add a retry path** — clicking Retry re-invokes the gateway call. If still failing, the banner stays. Telemetry / audit row: emit `AuditKind::GenesisFallbackUsed` (see brief 6 below for the new kind) every time the stub fires.

### Phase 3 — Idempotency key

9. **Generate a client-side UUID v7** per hire-flow lifecycle (set on Step 1 first-load; persisted across Step 2-6 in the draft). Use `crypto.randomUUID()` + a v7 wrapper (or import `uuid/v7` if `web/package.json` already has uuid).

10. **Include `idempotency_key` in the gateway hire POST** body. Gateway side: `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` accepts the field, de-dupes against an existing row keyed on `(tenant_id, idempotency_key, created_at_micros > now - 1h)`. If a duplicate is detected, return the EXISTING worker_id with a `200 OK` + body `{ duplicate_of: <id>, message: "Already hired (idempotent)" }`.

11. **Negative test**: SPA submits the same idempotency_key twice in a row → exactly 1 worker row in the DB. Plus the second response includes `duplicate_of` field.

### Phase 4 — Audit kind (defer or include)

12. **Optional**: add `AuditKind::GenesisFallbackUsed` (`"genesis.fallback_used"`) via the 4-site recipe so operators see when their workspace's Genesis is failing. **Recommend including** — 25 LOC delta, real operational visibility. If declining, scope-defer to a sibling brief named `mission-genesis-fallback-audit-kind`.

## Files you may touch (verified to exist on `2fa4baf1`)

- **MODIFY** `web/lib/hire/draft-types.ts` — field alignment.
- **MODIFY** `web/lib/hire/use-genesis-worker-draft.ts` — return `{ source, error }`; idempotency_key handling.
- **MODIFY** `web/lib/hire/genesis-stub.ts` — stays as-is, but mark deprecated for v2 (remove client stubs entirely once gateway reliability is proven).
- **MODIFY** `web/lib/api/workers.ts` — snake↔camel mapping at boundary; idempotency_key in POST body.
- **NEW** `web/lib/hire/__tests__/workerdraft-schema-shape.test.ts` — structural alignment test.
- **MODIFY** `web/app/(hire)/hire/step/3/page.tsx` (or actual location of Step 3 — verify with `rg -ln 'STUB_QUESTIONS' web/app/`) — fallback banner + Retry button.
- **MODIFY** `web/tests/e2e/hire-flow-step3-drafting.spec.ts` — extend to cover (a) live-gateway-success path, (b) stub-fallback-with-banner path, (c) idempotency-key roundtrip.
- **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` — accept + dedupe `idempotency_key`.
- **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_workers_hire_tests.rs` — extend with idempotency tests.
- **MODIFY** `crates/argo-audit/src/event.rs` + `crates/argo-audit/src/event_tests.rs` — IF including `GenesisFallbackUsed`, the 4-site recipe.

## Files you must NOT touch

- `crates/argo-genesis/src/*` core — the schema is owned there; this brief aligns the SPA to it, doesn't restructure Genesis.
- `crates/argo-store/migrations/*.sql` — no schema change required (idempotency_key lives in audit row payload or in a hire-request log if one exists; do NOT add a new table for v1).
- Any other hire-step page or worker-related surface — scope is Step 3 + the API client + the gateway hire endpoint.

## Pre-flight verification

```bash
# Confirm the wave-15 audit is still load-bearing — has anything closed this gap since the audit ran?
ls docs/skycot/audit/five-teammate-hire/ 2>&1 | head -3
git log --oneline origin/main | grep -iE 'workerdraft|five-teammate|mariah' | head -3

# Confirm gateway-side WorkerDraft struct location.
rg -n 'pub struct WorkerDraft' crates/

# Confirm SPA-side WorkerDraft interface.
grep -n 'interface WorkerDraft' web/lib/hire/draft-types.ts

# Confirm STUB_QUESTIONS callsite (Step 3 in the hire flow).
rg -ln 'STUB_QUESTIONS' web/

# Confirm uuid library available, or pick crypto.randomUUID approach.
grep '"uuid"' web/package.json
```

## Acceptance criteria

- [ ] Live-gateway hire success path produces a per-user-input WorkerDraft (different inputs → different drafts → different generated workers). Verified with a Playwright test that submits 2 different problem statements and asserts the resulting workers' personas differ.
- [ ] When gateway is unreachable, the user sees a visible WARNING banner saying generic questions are being used + a Retry button. Verified with a Playwright test that mocks the API endpoint to 500.
- [ ] Submitting Step 6 with the same idempotency_key twice produces exactly 1 worker row (gateway negative test).
- [ ] `web/lib/hire/__tests__/workerdraft-schema-shape.test.ts` passes: every gateway-side field is present in the TS interface (no drift).
- [ ] 5-teammate batch hire produces 5 distinct workers (different personas / names / voice profiles), NOT 5 "Mariah" clones. End-to-end Playwright test verifies.
- [ ] `pnpm typecheck` + `pnpm lint` + `pnpm test --run` clean (pre-existing flakes per HARD RULE 2 documented in PR body).
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `bash scripts/conductor-pre-merge-gate.sh` PASS (Tier-3 review because this touches the hire-flow which is procurement-grade).
- [ ] Every modified file < 500 LOC.
- [ ] No `unwrap` / `expect` outside `#[cfg(test)]`.

## Branch + PR

- Branch: `claude-web-hire-workerdraft-schema-align`
- PR title: `v1 — web/hire — fix 5-teammate Mariah-clone bug (WorkerDraft schema align + loud fallback + idempotency key)`

## Gotchas

- **Don't delete `genesis-stub.ts`**: it's the only way the hire flow handles a totally-unreachable gateway. The brief's goal is to make it OBSERVABLE when the stub fires, not to remove the safety net. v2 work (post-v1.0 launch) can remove client stubs entirely.
- **5-teammate batch is the killer test**: a Playwright run that opens 5 separate hire flows back-to-back with 5 different problem statements and asserts each generated worker is distinct. If 2 of 5 are identical, the bug is back.
- **Idempotency window**: 1 hour is the recommended default. Long enough to catch double-clicks + reload-storms; short enough that a user retrying tomorrow gets a fresh hire. Make it configurable via `ARGO_HIRE_IDEMPOTENCY_WINDOW_SECS` for operator tuning.
- **The audit's "Mariah" framing**: the audit found that the stub-generated worker's name is sometimes literally "Mariah" (because the persona-generation seed for stub answers happens to produce that name deterministically). The fix isn't to randomize the name — it's to never fall to stub silently. With the loud banner + observability, "Mariah" disappears as an issue because users notice when they're getting a stub.

## References

- Audit: `docs/skycot/audit/five-teammate-hire/` + wave-15 final verdict §4.
- Sibling code:
  - `web/lib/hire/draft-types.ts` — TS WorkerDraft interface.
  - `web/lib/hire/genesis-stub.ts` — the silent fallback.
  - `web/lib/hire/use-genesis-worker-draft.ts` — the hook that swaps between gateway + stub.
  - `crates/argo-cli/src/handlers/gateway/routes_workers_hire.rs` — gateway hire endpoint (post-#1860 era).
- Pattern reference: `claude-rebase-pr-1835-cookies` (#1866) for snake↔camel mapping + the way the gateway emits compact JSON.
- Audit kind 4-site recipe: `mission-onboarding-audit-variants` (#1920 IN_REVIEW) for the latest cleanest variant-add diff.
