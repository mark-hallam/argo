# web-trust-test-mock-recompute-fix

**Authored against**: `origin/main` `8b216295` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-trust-test-mock-recompute-fix`.

## Summary

Declared as a follow-up by #1957 (web-tooltip-primitive) PR body. Pre-existing test failures in `web/lib/api/__tests__/Trust.test.tsx` (or similar — verify the exact path) involve a mock for the trust-engine recompute path that wasn't updated when #1949 (`web-worker-trust-promotion-engine`) shipped the new canonical recompute API. The mock returns the old shape; the test assertions expect the new shape.

This is the "Trust.test.tsx failures" item in the rolling boundary-flake list.

## Acceptance

- [ ] Identify the failing Trust test(s) — likely under `web/lib/api/__tests__/` or `web/components/skycot/`. Run `pnpm -C web test -- Trust` to see the actual file paths + failure messages.
- [ ] For each failure, decide:
  - **Mock-out-of-date** — update the mock to return the post-#1949 canonical recompute response shape.
  - **Assertion-out-of-date** — update the assertion to match the post-#1949 reality.
  - **Real bug** — if the failure surfaces a real bug, fix it AND cite the audit chain.
- [ ] `pnpm -C web test -- Trust` returns 0 failures after the fix.
- [ ] No regression on other tests — `pnpm -C web test` full suite has 0 NEW failures (the broader 5-7 pre-existing flake list shrinks by the count of Trust failures fixed here).
- [ ] PR body shows the before/after failure count + the mock/assertion diff.
- [ ] Test-only fix — no production code changes unless a real bug surfaces.

## May touch

- `web/lib/api/__tests__/Trust*.test.{ts,tsx}` (or wherever the failing tests live)
- `web/components/skycot/**/Trust*.test.tsx` if the failures are in component tests
- A mock helper if the mock is shared

## Must NOT touch

- `argo-trust-engine` crate (#1949 settled).
- Production trust-display components (#1949 + downstream UIs are settled).
- Non-Trust tests in the suite.

## Notes

- Pre-flight: `pnpm -C web test -- Trust 2>&1 | head -50` to see the actual failure messages first.
- This is part of the rolling boundary-flake sweep started by `claude-web-test-flake-sweep-wave-5` (#1952) — that sweep deferred this specific item because the fix requires consuming #1949's new mock shape.
- Sizing: 50–150 LOC.
