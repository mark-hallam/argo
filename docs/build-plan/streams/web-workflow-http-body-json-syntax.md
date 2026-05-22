# web-workflow-http-body-json-syntax

**Owner**: claude.
**Depends on**: `web-workflows-http-and-code-nodes` MERGED (✅ landed via #2011 on 2026-05-22).
**Authored against**: origin/main `862d45a3b` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-http-body-json-syntax`.

🟢 **P3 BUILDER POLISH.** Source: #2011 PR-body deferral ("Body textarea (monospace, plain — JSON-aware syntax highlighting deferred)"). #2011 shipped HTTP step config with a plain monospaced `<textarea>` for the request body. Add lightweight JSON syntax awareness: pretty-print on blur, error-underline on bad JSON, and inline parse-error message.

## Scope

In `web/app/(app)/workflows/[verticalId]/[name]/build/step-config-http.tsx`, extend the request-body editor with:

1. **Pretty-print on blur.** When focus leaves the textarea AND the content parses as valid JSON, replace the textarea value with `JSON.stringify(parsed, null, 2)`. Skip if content is empty or doesn't look like JSON (first non-whitespace char ∈ `{[`).
2. **Inline parse-error message.** Below the textarea, show a one-line error in red when the body looks like JSON (starts with `{` or `[`) but doesn't parse. Message format: `JSON parse error at line N: <native message>`. Hidden when parse succeeds OR when content is empty / not JSON-shaped.
3. **Optional**: lightweight syntax tinting via a CSS class that bumps font color slightly for `"strings"`, `numbers`, and `true/false/null`. **This is a stretch goal; skip if it adds dependencies.** A plain textarea with the two features above is the minimum viable.

Do NOT pull in Monaco for this. Monaco lands in the sibling Code-step brief (`web-workflow-monaco-editor-for-code-step`). The HTTP body editor stays lightweight — keeping the SPA cold-start under budget is the constraint.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/step-config-http.tsx` — extend the body-editor section.
- `NEW: web/lib/builder/json-helpers.ts` — small `tryParseJson` + `prettyPrint` helpers (extracted so they're reusable by future briefs).
- `NEW: web/__tests__/builder/step-config-http-json.test.tsx` — vitest cases: pretty-print on blur, parse error inline, no error on empty / non-JSON, no error on valid JSON.

## Files you must NOT touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — locked from #2013's extract.
- The Code step config editor — separate brief.
- The HTTP step's other fields (URL / headers / auth / timeout) — only the body section changes here.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] Pasting `{"a":1,"b":2}` into the body and blurring reformats to `{\n  "a": 1,\n  "b": 2\n}`.
- [ ] Typing `{a:1}` (invalid JSON — missing quotes) shows inline `JSON parse error at line 1: <message>`.
- [ ] Empty body shows no error.
- [ ] `Hello world` (plain text, no JSON shape) shows no error.
- [ ] Valid JSON shows no error and pretty-prints on blur.
- [ ] No regression on HTTP step submit (form value carries the pretty-printed body forward).
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm lint` clean.
- [ ] `pnpm test --run` passes the new test file.
- [ ] Bundle size impact noted in PR body: this brief MUST NOT add deps. Native `JSON.parse` only.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-web-workflow-http-body-json-syntax`
- PR title: `web(workflow): JSON-aware body editor on HTTP step (web-workflow-http-body-json-syntax)`

## Notes for the implementer

- ~100-150 LOC expected.
- The "looks like JSON" heuristic is just "first non-whitespace char is `{` or `[`". Don't try to be cleverer.
- Native `JSON.parse` gives line-1-only error positions on V8/JSC; that's fine — the message itself is what's useful. Don't pull in a JSON parser dep.
- The optional syntax tinting can use a sibling overlay `<pre>` rendered behind/under the textarea (the classic "fake-syntax-highlight on textarea" pattern). Skip if it makes the test brittle.
- The sibling Monaco brief covers the Code step. Don't unify them; the two surfaces have different weight budgets.
