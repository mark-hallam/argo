# web-workflow-monaco-editor-for-code-step

**Owner**: claude.
**Depends on**: `web-workflows-http-and-code-nodes` MERGED (✅ landed via #2011 on 2026-05-22).
**Authored against**: origin/main `862d45a3b` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-monaco-editor-for-code-step`.

🟢 **P3 BUILDER POLISH.** Source: #2011 PR-body deferral ("Monospaced textarea (Monaco deferred — keeps SPA bundle small for v1)"). #2011 shipped HTTP + Code step kinds with a plain `<textarea>` for the code body. Replace that with the Monaco editor for syntax highlighting + line numbers + autocomplete on JS Node 20 / Python 3.12 code.

## Scope

In `web/app/(app)/workflows/[verticalId]/[name]/build/step-config-code.tsx` (the code-step config editor that ships in #2011), replace the plain `<textarea>` with a Monaco editor. Wire it as a **dynamic import** so the Monaco bundle is only fetched when the user actually opens a code-step editor — keeps the cold SPA start under the current page-weight budget.

Use `@monaco-editor/react` (the maintained React wrapper, ~5KB itself; lazy-loads the ~3MB monaco runtime). Set the language based on the step's `language` field (`javascript` or `python`). Configure: tab size 2 spaces, no minimap, line numbers on, word wrap off, theme matching the workspace (light by default; if a dark-mode toggle exists, follow it).

On change, propagate the value via the existing `onStepChange` callback. Empty-code inline error from #2011 stays — keep it.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/step-config-code.tsx` — swap `<textarea>` for `<Suspense>` + `<MonacoEditor>` (lazy import).
- `web/package.json` — narrowly scoped: add ONE dependency `@monaco-editor/react` (latest stable). No other dep churn.
- `web/pnpm-lock.yaml` — regenerated to include the new dep.
- `NEW: web/components/builder/MonacoCodeEditor.tsx` (or similar — the lazy-loaded wrapper component if direct dynamic import in step-config-code.tsx is messy).
- `NEW: web/__tests__/builder/step-config-code-monaco.test.tsx` — vitest cases: editor renders for JS + Python; onStepChange fires on edit; empty-code error still surfaces.

## Files you must NOT touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — locked from #2013's extract.
- The HTTP step config editor — separate brief (`web-workflow-http-body-json-syntax`) covers JSON polish there.
- Any other step config editor (api / mcp / tool / etc.) — out of scope.
- `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts` — narrowly scoped, ONLY if the language field type needs widening; verify first.

## Acceptance criteria

- [ ] Code step config opens a Monaco editor with syntax highlighting for both JS Node 20 (`javascript`) and Python 3.12 (`python`).
- [ ] Editor is dynamically imported — `pnpm build` shows no Monaco bytes in the main bundle (verify via Next bundle analyzer output in PR body, or by `pnpm build` + grep build manifest).
- [ ] Empty-code inline error from #2011 still surfaces.
- [ ] `onStepChange` fires on every edit (test covers this).
- [ ] No regression on JS/Python language switch — toggling rebinds the editor's language without losing the code.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm lint` clean.
- [ ] `pnpm test --run` passes on the new test file + does not regress prior tests.
- [ ] Manual smoke noted in PR body: open builder, drop a Code node, open its config, type `console.log("hi")` for JS / `print("hi")` for Python, confirm highlighting.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-web-workflow-monaco-editor-for-code-step`
- PR title: `web(workflow): Monaco editor for Code step config (web-workflow-monaco-editor-for-code-step)`

## Notes for the implementer

- ~200-300 LOC expected. Most of the work is bundle-config correctness (the dynamic import boundary) and the test.
- The Monaco runtime is ~3MB. Dynamic import is the critical pattern; if you ship it eagerly, the cold SPA start regresses. Verify with Next bundle analyzer or by checking the `_next/static/chunks/` build output that monaco lands in its own chunk.
- Don't introduce a worker for Monaco's language services in v1 — v0 syntax highlighting + line numbers is enough. Workers are an optimization for the next pass.
- If `@monaco-editor/react` SSR-pukes (it does in some setups), wrap the dynamic import with `ssr: false` per Next's docs.
- Don't touch the HTTP step config — `web-workflow-http-body-json-syntax` is its sibling brief.
