# web-spa-no-server-imports-lint

**Owner**: claude.
**Depends on**: `web-auth-config-server-only-split` MERGED (✅ via #2027).
**Authored against**: origin/main `cbd19bbb3` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-spa-no-server-imports-lint`.

🟡 **P2 RECURRENCE-PREVENTION.** Source: tick-40 closure of P0 #2027 (cookies-in-client-bundle). The bug existed because there's no lint that prevents `next/headers` (or any other server-only API) from being imported by a Client Component file. #2027 added a unit test that pins `auth.config.ts` specifically, but the same class of bug can land in any other client file tomorrow. Add a workspace-wide lint that REFUSES `next/headers` imports in client-bundled files.

## Scope

Add an ESLint rule (preferred) or a grep-based CI check (fallback if ESLint rule is hard) that fails the build when any file under `web/` that is, or transitively becomes, a Client Component imports from `next/headers` (or `next/cookies` if that's a thing). The rule must catch BOTH:
1. Files explicitly marked `"use client"` at the top.
2. Files transitively bundled by a Client Component (these are harder to detect statically; pragmatic compromise: any file not explicitly marked `"use server"` AND not `auth.ts` / `middleware.ts` / `route.ts` / `*.server.ts` is treated as client-eligible).

The brief delivers ONE OF (worker chooses, document in PR body):
- **(A) ESLint rule** — custom rule in `web/.eslintrc.js` (or equivalent) flagging `next/headers` import in non-server-only files. Hooks into the existing `pnpm lint` so the gate runs every PR.
- **(B) Grep CI step** — a small script `scripts/check-no-server-imports.sh` that greps `web/` for `from "next/headers"` in non-allow-listed files and exits 1 on match. Wired into the existing pre-merge / lint workflow.

Option (A) is preferred (proper static-analysis); Option (B) is acceptable if (A) requires too much ESLint plugin scaffolding for v1.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/.eslintrc.js` (or `web/.eslintrc.json` / `web/eslint.config.mjs` — verify which one is canonical) — narrowly scoped: add the rule entry.
- `NEW: web/eslint-rules/no-server-imports-in-client.js` (if going Option A and a custom rule plugin is the chosen path) — the rule implementation.
- `NEW: scripts/check-no-server-imports.sh` (if going Option B).
- `web/package.json` — narrowly scoped: if Option A requires a new dev-dep (e.g. an ESLint plugin starter), ONE entry only.
- `NEW: web/__tests__/lint/no-server-imports.test.ts` — unit test that asserts the rule fires on a known bad sample + does NOT fire on `auth.ts` / `route.ts` / `*.server.ts`.

## Files you must NOT touch

- Any client component source — only the lint + scaffold.
- `web/auth.config.ts` — locked from #2027.
- `web/auth.ts` — locked from #2027.
- Gateway code.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] PR body explicitly states Option A or Option B and one-sentence justification.
- [ ] A planted-bad sample file importing `next/headers` from a client-bundled context triggers the lint, fails the build, and points at the offending import.
- [ ] `auth.ts`, `middleware.ts`, files matching `*.server.ts`, and `app/api/**/route.ts` are exempt.
- [ ] An attempt to reintroduce the #2027 bug (commenting in `cookies` to `auth.config.ts`) would now be caught at lint time, not at runtime cold start.
- [ ] `pnpm lint` clean on current main.
- [ ] If Option A: zero new ESLint warnings introduced on existing files. If existing files trip the rule, document them in the PR body as out-of-scope follow-ups.
- [ ] If Option B: script runs in <2 seconds.
- [ ] `pnpm typecheck` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-web-spa-no-server-imports-lint`
- PR title: `web(lint): refuse next/headers in client-bundled files (web-spa-no-server-imports-lint)`

## Notes for the implementer

- ~150-250 LOC expected. ESLint rule scaffold is ~80 LOC; grep script is ~30 LOC; test is ~50-100 LOC.
- Don't try to be ambitious. The rule's job is to flag `next/headers` imports specifically. Generalize later if needed; this brief is targeted at #2027's bug class.
- The "client-bundled file" detection heuristic: assume EVERY file under `web/app/` is potentially client-bundled UNLESS it (a) starts with `"use server"` directive or (b) is named `route.ts` (server-side route handler) or (c) is named `*.server.ts`. This is conservative — it'll false-positive on some server-only files that aren't marked. The fix for false-positives is to add `// allowlist:no-server-imports-in-client` comment to specific files; document the convention in the PR body.
- If ESLint plugin scaffolding is unfamiliar, go Option B (grep script). A working grep script that catches the regression is more valuable than a half-working ESLint rule.
