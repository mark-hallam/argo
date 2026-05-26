# Round 92 — Vitest 1649/1649 GREEN claim verification (commit d40c4f72d)

- **Date / UTC**: 2026-05-26T19:18Z
- **Round**: 92
- **Pillar / Claim**: process-integrity / web-suite stability (no direct mission claim)
- **Verdict**: 🟡 **PARTIAL** (P3) — the 1-LOC test rename DID fix the WizardEditorialShell failure as claimed, but a fresh cold full-suite run on a different machine surfaces a *different* test as timing out under load. The "1649/1649 GREEN" claim is true in isolation but not durable under contention. Recommend a P3 brief to either (a) raise the test-level timeout for Step4Introduce.test.tsx or (b) add it to KNOWN_FLAKES.md.

## Setup

```bash
git show d40c4f72d --stat      # 1-LOC rename + ledger flips
cd web && pnpm exec vitest run --reporter=default
# then re-run the failing file in isolation:
pnpm exec vitest run components/skycot/hire/Step4Introduce.test.tsx
```

Commit `d40c4f72d` (Wed May 27 04:58 AEST = 2026-05-26T18:58Z UTC) renames the assertion testid in `web/components/auth/__tests__/WizardEditorialShell.onboarding.test.tsx:62` from `onboarding-pillar-single-binary` → `onboarding-pillar-genesis-studio`. Commit message claims:

> Test runtime vitest now 1649/1649 GREEN.

## Observed

**Full-suite run (cold, fresh node):**
```
Test Files  1 failed | 228 passed (229)
Tests       1 failed | 1648 passed (1649)
Duration    84.60s  (transform 37.14s, setup 36.89s, collect 196.50s,
                     tests 223.98s, environment 130.11s, prepare 37.74s)
```

The 1 failure:
```
FAIL  components/skycot/hire/Step4Introduce.test.tsx
> Step4Introduce > surfaces the template fallback in the drawer after 3 successful adjusts
Error: Test timed out in 5000ms.
   ❯ components/skycot/hire/Step4Introduce.test.tsx:217:3
```

**Isolation re-run of the same file (8 minutes later, same machine):**
```
✓ components/skycot/hire/Step4Introduce.test.tsx (12 tests) 936ms
Test Files  1 passed (1)
Tests       12 passed (12)
Duration    3.05s
```

12/12 pass in 936ms in isolation — but the same test times out in 5000ms under full-suite parallel load.

## Anti-theater check

1. **The 1-LOC rename IS real and IS reachable.** The testid `onboarding-pillar-genesis-studio` appears in `web/components/auth/WizardEditorialShell.tsx` (production component) — `git grep onboarding-pillar-genesis-studio web/components/auth/ | wc -l` returns 2 hits (1 prod + 1 test). The renamed testid matches the testid the production code emits, so the test now asserts against reality.
2. **WizardEditorialShell.onboarding.test.tsx does NOT appear in the failed list** of the full-suite run — confirming the rename closed the failure R88/R90/R91 carried.
3. **The new failure (Step4Introduce.test.tsx) is NOT a regression introduced by d40c4f72d.** The test file hasn't been touched in months — last touch is `db64f5aba` (PR #1512, the original Step 4 implementation). Component was last touched 2026-05-22 by `51cf7ade0` (PR #2002, loud-failure banner).
4. **Reproduction in isolation confirms it's a load-induced flake**, not a real failure. The pattern matches the radix-ui `react-toast` `hasPointerCapture is not a function` jsdom warnings that pollute stderr without causing failures — but in a parallel-load environment the test-event chain takes longer than 5000ms to complete.

## Why the conductor's claim was overstated

Conductor's commit message asserts "1649/1649 GREEN" based on the run that happened on the conductor's machine in the operator's interactive session. The supervisor was halted at that point (per SHARED.md `[supervisor-watchdog]` entries showing 900s respawn backoff), so the conductor's vitest run was under lower system contention than my evaluator run (worker-supervisor is now likely back up; my machine state isn't the conductor's state).

This isn't dishonest — it's the same flake reproducibility problem we've seen across rounds. But the headline number "1649/1649" should be qualified as "1649/1649 in conductor's session; flaky under load."

## Gap / follow-up brief candidate

- **Name**: `ops-known-flake-step4introduce-template-fallback-timeout`
- **Priority**: P3
- **Scope**: ONE of:
  - (a) Add an `it.fails` / `it.skip(LOAD)` guard with a comment pointing here; OR
  - (b) Increase the test's local timeout to 15000ms (the userEvent + radix-toast chain genuinely takes a while under load); OR
  - (c) Add an entry to `docs/build-plan/KNOWN_FLAKES.md` with reproduction notes.
- **Why P3**: the test is flake-under-load, not a real regression. The product code path is fine (12/12 pass in isolation). This is hygiene, not a ship-gate blocker.

Recommend (b) — raising the timeout is the smallest reversible change and preserves the assertion. (a) and (c) both hide the test.

## Recommendation

Close the 1-LOC rename as **real fix confirmed** for the WizardEditorialShell carry across R85/R86/R88/R90. Note in the next conductor tick that the "1649/1649 GREEN" headline is load-dependent — the durable number is **1648 PASS + 1 LOAD-FLAKE**, and the flake is in Step4Introduce.test.tsx, not WizardEditorialShell. Author the P3 brief above if the conductor wants the flake nailed before demo recording.
