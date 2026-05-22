# claude-dev-safe-check-deps-present

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `02fad8a06` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-dev-safe-check-deps-present`.

🟡 **P1 DEV HYGIENE — recurring footgun. Round 1 + round 5 both hit it.** Source: evaluator round-5 handoff 2026-05-23 04:10 UTC.

**Problem.** Twice in 36h, dependencies declared in `web/package.json` were absent from `web/node_modules` on the operator's dev machine:

- **Round 1 (2026-05-22)**: `@radix-ui/react-tooltip@^1.2.8` (from PR #1957) — installed in package.json, missing on disk.
- **Round 5 (2026-05-23)**: `@monaco-editor/react@^4.6.0` + 6 transitive deps — same shape.

In both cases the lockfile (`pnpm-lock.yaml`) is up-to-date. `pnpm install --prefer-offline` resolves in <2 seconds. **The gap is that nothing in the cold-start dev script verifies the deps are actually on disk.** Result: Turbopack throws `Module not found` and shows a dev-error overlay across every page in the SPA. Demos and Playwright runs blow up.

This is a recurring incident — twice in 36h is a pattern, not a fluke. Add a guard.

## Scope

Extend `scripts/dev-safe-check.sh` with a step that:

1. Reads each top-level entry in `web/package.json`'s `"dependencies"` block (NOT `"devDependencies"` — those are dev-only and a missing one wouldn't break the SPA).
2. For each dep `pkg`, confirms `web/node_modules/<pkg>/package.json` exists (scoped pkgs at `web/node_modules/@scope/name/package.json`).
3. If any are missing, print a loud red message naming each missing dep + the one-liner fix:
   ```
   ❌ Missing dependencies in web/node_modules:
        @monaco-editor/react
        @some-other/pkg
   Fix:  cd web && pnpm install
   ```
4. Exit 1 on missing-dep state. Exit 0 if all present.

If the script doesn't exist yet, create it at `scripts/dev-safe-check.sh` with this check as its first step. If it does exist, add this check as an additional step (don't break existing behaviour).

### Optional: Rust symmetry

If it fits the same script's character budget (~30 LOC extra), do the same check for `crates/*/Cargo.toml` listed deps absent from `Cargo.lock`'s `[[package]]` section. **Default is web-only.** The Rust path is much less prone to this exact failure mode (cargo runs at build time, not dev-server-overlay time), so it's nice-to-have not load-bearing.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `scripts/dev-safe-check.sh` — extend or create, ~30 LOC of bash.
- `docs/skycot/operator/dev-runbook.md` (or wherever the dev-safe-check is documented) — narrowly scoped: 1-2 lines pointing at the new check.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Production crates.
- Web app code.
- The gateway.
- `pnpm-lock.yaml` — leave the lockfile alone.

## Acceptance criteria

- [ ] `bash scripts/dev-safe-check.sh` exits 0 on a clean tree where all deps are present.
- [ ] Temporarily delete `web/node_modules/@monaco-editor` and rerun — exits 1 with a message naming `@monaco-editor/react`.
- [ ] Restore via `pnpm install` and rerun — exits 0.
- [ ] CI's existing dev-safe-check job (if it runs this script) catches the missing-dep before the SPA boots.
- [ ] shellcheck clean on the modified script.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-dev-safe-check-deps-present`
- PR title: `ops(dev-safe-check): verify web/node_modules matches package.json before SPA boot (claude-dev-safe-check-deps-present)`

## Notes for the implementer

- ~30-50 LOC of shell. Use `jq` to parse package.json (already a workspace dep).
- **The two incidents to cite in the PR body**: round 1 missing `@radix-ui/react-tooltip`, round 5 missing `@monaco-editor/react`. Quote the evaluator round-5 handoff verbatim.
- **Don't make it too noisy in the happy path** — when all deps are present, print one line confirming "node_modules ok (N deps)" and move on. Silent success would be hostile to the operator who's running the script and wants confirmation.
- The fix command shown in error output MUST be `cd web && pnpm install` (not just `pnpm install` from repo root — pnpm workspaces work from the workspace root, but the operator's failure mode in both incidents was running `pnpm dev` from `web/` without re-installing first).
- **Scope discipline**: this is the smallest possible guard. Don't over-engineer with watcher daemons, pre-commit hooks, or auto-install. Just check + exit on miss.
- This brief is the evaluator round-5 recommended fix paired with brief A.
