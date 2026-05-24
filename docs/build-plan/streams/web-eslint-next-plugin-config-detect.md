# web-eslint-next-plugin-config-detect

**Owner**: claude.
**Authored against**: origin/main `ad1e38ba2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-eslint-next-plugin-config-detect`.

## Why

`pnpm -C web build` currently emits this warning on `origin/main`:

```text
The Next.js plugin was not detected in your ESLint configuration.
```

The app builds past the warning, but this weakens lint coverage over App Router
and framework-specific rules exactly where the web surface is evolving fastest.
The current `web/eslint.config.mjs` is intentionally minimal and appears to
have drifted behind the Next 15 flat-config expectations.

## Scope

Restore framework-aware ESLint detection for the `web/` app:

- wire the Next.js ESLint plugin/config into the flat config actually used by
  `pnpm -C web build` / lint
- preserve the existing local ignores and custom `no-alert` rule
- avoid broad rule churn; this brief is config detection and parity only

## Files you may touch

- `web/eslint.config.mjs`
- `web/package.json` only if a script or dependency pin needs a narrow update
- `web/next.config.ts` only if detection requires an explicit bridge and the
  diff stays tightly scoped

## Files you must NOT touch

- application code under `web/app/**`
- seed/sample content under `web/lib/sample-*`
- unrelated test fixtures

## Acceptance criteria

- [ ] `pnpm -C web build` no longer prints the Next.js ESLint plugin warning
- [ ] existing local ignores remain intact
- [ ] the custom `no-alert` enforcement still applies
- [ ] no unrelated new lint failures are introduced by config churn alone

## Branch + PR

- Branch: `claude-web-eslint-next-plugin-config-detect`
- PR title: `web: restore Next.js ESLint plugin detection (web-eslint-next-plugin-config-detect)`

## Notes

- Keep the fix small. This is not the broader lint-stack migration.
- If the correct fix is to compose the official Next flat config, document the
  precedence clearly in the config file comments.
