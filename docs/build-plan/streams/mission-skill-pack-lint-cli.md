# mission-skill-pack-lint-cli — `argo skill pack lint <path>` (post-v1 polish P1)

**Owner**: claude. **Depends on**: none (single-crate, mostly).
**Authored against**: origin/main `e2fe4b70` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-pack-lint-cli`.
**Source**: post-v1 polish — vertical-pack authors need a lint pass before publishing to skycot-packs.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — bad packs at publish time leak into worker dispatch — fail fast.
- **Repeatability** (#3) — pack authors get same validation results regardless of pack source.
- **Extensibility** (#6) — vertical-template system is core; ship-with-tooling.

## Scope

P1. New CLI subcommand:
```bash
argo skill pack lint <path>        # Lint a pack directory
argo skill pack lint <path> --strict   # Treat warnings as errors
argo skill pack lint <path> --json     # Machine-readable output
```

Checks (informed by `crates/argo-vertical/src/eval_score.rs` and existing pack structure under `packs/`):

| # | Check | Severity |
|---:|---|---|
| 1 | `pack.toml` exists + parses + has `name`, `version`, `verticals` | error |
| 2 | `worker.toml` exists + parses (if pack contains worker defaults) | error |
| 3 | `memory_schema.json` parses + each field has a `type` | error |
| 4 | Each Skill IR (`.json` under `skills/`) parses | error |
| 5 | Each Skill IR's `tool` field references an existing tool name or known MCP connector | warn |
| 6 | Each Skill IR's `capabilities` field is a valid `CapabilitySet` | error |
| 7 | Eval fixtures (`evals/*.json`) parse + each has `schema_version >= 1` | error |
| 8 | Eval fixtures' `expected_outcome` matches the Skill IR's declared shape | warn |
| 9 | `README.md` exists | warn |
| 10 | No file > 500 LOC (architecture invariant) | warn |
| 11 | Names follow `kebab-case` (pack name, skill names) | warn |
| 12 | `version` is semver | error |

Output format (text mode):
```
packs/gtm-engineer/
  ✅ 12 checks passed
  ⚠️  2 warnings: ... 

packs/marketing-intern/
  ❌ 1 error: memory_schema.json: missing "type" on field "campaign_id"
  ⚠️  3 warnings: ...
```

Exit codes: 0 = clean, 1 = warnings only (no `--strict`), 2 = error or `--strict + warning`.

LOC: ~300.

## Files you may touch

- `crates/argo-cli/src/cli_skill_cmds.rs` (existing) — extend with `Pack { Lint { path, strict, json } }` variant. ≤25 modified LOC.
- `crates/argo-cli/src/handlers/skill_pack_lint.rs` (NEW) — handler. ≤180 LOC.
- `crates/argo-cli/src/handlers/skill_pack_lint_tests.rs` (NEW) — golden-fixture tests against `packs/gtm-engineer/`, `packs/marketing-intern/`, plus 2 deliberately-broken pack fixtures. ≤120 LOC.
- `crates/argo-skill/src/pack_lint.rs` (NEW) — pure logic: `lint_pack(path) -> LintReport`. ≤180 LOC.
- `crates/argo-skill/src/pack_lint_tests.rs` (NEW). ≤120 LOC.
- `crates/argo-skill/src/lib.rs` — re-export. ≤3 modified LOC.
- `docs/skycot/cli-reference.md` — add `skill pack lint` row. ≤8 modified LOC.
- `docs/skycot/operator/skill-pack-authoring.md` (existing or NEW) — add a "Lint before publish" section. ≤30 modified LOC.

## Files you must NOT touch

- Existing pack fixtures under `packs/` — read-only. Worker MAY add 2 deliberately-broken test fixtures under `crates/argo-skill/test-fixtures/broken_pack_*/`.
- `argo-vertical::eval_score` — reuse its public API for the eval-shape match check; don't fork the logic.
- The pack-loader runtime (`argo-skill::loader`) — lint is offline; loader is runtime. Different layer.

## Acceptance criteria

- [ ] `argo skill pack lint packs/gtm-engineer/` returns exit 0 with all-checks-passed output.
- [ ] `argo skill pack lint <broken-fixture>` returns exit 2 with specific error messages.
- [ ] `--strict` flag escalates any warning to exit 2.
- [ ] `--json` flag emits `{ pack, checks: [{ id, severity, message }], summary: { errors, warnings } }`.
- [ ] All 12 checks have a test case demonstrating they fire.
- [ ] Output is line-buffered (operator sees progressive lint output on multi-pack runs).
- [ ] `cargo test -p argo-skill -p argo-cli` passes.
- [ ] `cargo clippy -p argo-skill -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-skill-pack-lint-cli`
- Title: `feat(cli): argo skill pack lint — pre-publish pack validator (mission-skill-pack-lint-cli)`
- Worker verification: crate-scoped clippy + tests + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Capabilities have a schema.** `CapabilitySet` is defined in `argo-types::capability`; use the deserialize path to validate, don't reimplement.
- **Tool-name reference check is warning, not error.** A pack may declare a tool that's only available behind a feature flag; the runtime loader handles missing tools gracefully.
- **No network calls.** Lint is offline. Don't fetch the marketplace.
- **Don't lint `.lock`-style files.** Skip any file matching `*.lock`, `*.tmp`, `*.swp`.
- **Output coloring** — auto-detect `isatty(stdout)`; use ANSI codes only when terminal, never when piped to file.
- **Semver parser** — use `semver` crate (already in workspace).
