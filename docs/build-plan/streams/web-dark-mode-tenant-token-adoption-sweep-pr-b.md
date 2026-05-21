# web-dark-mode-tenant-token-adoption-sweep-pr-b

**Authored against**: `origin/main` `93c013f8` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-dark-mode-tenant-token-adoption-sweep-pr-b`.

## Summary

PR-B of the dark-mode-tenant-token-adoption-sweep brief. PR-A (#1946) shipped the mechanical 1:1 swaps on 9 files + the `--gradient-end` token. PR-B is the **per-callsite verified** pass on the remaining 10–12 files where each color callsite needs semantic review (conditional/composited values that can't be 1:1 swapped).

Pre-condition: #1946 (PR-A) MERGED ✅.

## Acceptance

- [ ] All 10–12 remaining callsites identified by the brief's hex-grep (PR-A's `rg` excluded these) are swapped to the right tenant token. Each swap is a deliberate choice, not mechanical.
- [ ] PR body lists each callsite with: file:line, hex value, chosen token, 1-line rationale.
- [ ] Cases where the hex literal is semantically correct (e.g. SKYCOT brand-locked gradient endpoints in marketing-only contexts) are listed as INTENTIONAL-KEEP in the PR body with a 1-line justification.
- [ ] Playwright visual-regression spec covers light + dark theme on the touched routes (the brief recommended this — verify the spec runs on at least the 4 highest-impact routes).
- [ ] Light + dark renders verified manually (or by Playwright screenshot diff) — paste a diff/note in the PR body.
- [ ] `pnpm -C web build` clean (PR-A claims this; verify it still is).
- [ ] All existing token-related tests pass.

## May touch

- The 10–12 files identified in the original brief's PR-B section (read `docs/build-plan/streams/web-dark-mode-tenant-token-adoption-sweep.md` for the canonical list).
- New Playwright spec under `web/tests/e2e/` (per the original brief's recommendation).

## Must NOT touch

- The 9 files PR-A already swept (those are done).
- `web/app/globals.css` — the canonical tokens are settled; don't add new ones in PR-B.
- The brand-loud-locked surfaces (audit page, workers page, cohorts, hire/done, onboarding) — those have brand-locked palettes per operator design memories.

## Notes

- Original brief: `docs/build-plan/streams/web-dark-mode-tenant-token-adoption-sweep.md` (PR-A section is done; PR-B section is what this brief implements).
- PR-A PR body lists the 14 residual hex values across the broader codebase — most belong to PR-B (`#ece9f5`, `#fef7f0`, `#5b21b6`, `#fee2e2`, `#3f3f46`, `#d8c7f7`, `#f5c5d8`, `#f4f0fa`, `#dc2626`, `#b91c1c`, `#15803d`, `#6366f1`, `#f4f4f4` — and possibly `#fafafa`).
- Sizing: 200–400 LOC.
- Auditor's earlier signal — tenant-theming is part of mission claim 6 (multi-channel approval procurement story).
