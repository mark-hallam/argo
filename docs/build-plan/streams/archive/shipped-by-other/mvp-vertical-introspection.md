# mvp-vertical-introspection — `argo vertical list/describe` + GET /verticals routes

**Owner**: claude. **Depends on**: nothing. **Tier 1 MVP — closes vertical-template surface.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

V1.0 ships with multiple demo verticals (finance-ar, healthcare-intake, real-estate-agent, legal-paralegal, recruiter, tax-prep, event-coordinator, product-manager). Operators have no programmatic way to see what's installed and what each vertical declares. This brief surfaces them via CLI + HTTP.

## Approach (concrete)

1. **CLI**: extend `VerticalCmd` (in `cli_command_enum.rs` or `cli_vertical_cmds.rs`) with two new subcommands:
   - `argo vertical list [--json]` — list all verticals from `verticals/` directory; default text output, `--json` emits structured array (per CLAUDE.md `JSON output for CLI list/show/status` rule).
   - `argo vertical describe <name> [--json]` — emit the vertical's manifest as readable text or canonical JSON (persona, system_prompt summary, action list, executor matrix).
   - Note: `argo vertical doctor <name>` already exists; reuse loader.
2. **HTTP**: add two read-only routes in `routes_meta_tools.rs` or sibling:
   - `GET /verticals` — returns array of `{name, version, description}`
   - `GET /verticals/<name>` — returns full manifest as JSON
3. **Store**: NO store changes — load from disk via `argo_vertical::registry::Registry::load_dir`.

## Files you may touch

- `crates/argo-cli/src/cli_command_enum.rs` OR `crates/argo-cli/src/cli_vertical_cmds.rs` (the latter if it exists post-split) — narrowly: `List` + `Describe` variants
- `crates/argo-cli/src/handlers/vertical.rs` — narrowly: handler functions
- `crates/argo-cli/src/handlers/gateway/routes_meta_tools.rs` — narrowly: 2 GET handlers
- `crates/argo-cli/src/handlers/gateway/routes.rs` — narrowly: route table entries
- `crates/argo-cli/tests/vertical_introspection_smoke.rs` (NEW, ≤ 200 LOC) — drives both CLI subcommands + GET via real binary

## Files you must NOT touch

- `argo-vertical` crate (loader/registry stays as-is — it's already capable)
- Any individual vertical's TOML or skill files
- Any other gateway route file beyond the entries

## Acceptance criteria

- [ ] `argo vertical list --json` emits an array
- [ ] `argo vertical describe finance-ar --json` emits the full manifest with all sections
- [ ] `GET /verticals` returns 200 with array
- [ ] `GET /verticals/finance-ar` returns 200 with full manifest; `GET /verticals/nonexistent` returns 404
- [ ] Smoke test passes
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean

## Branch + PR

- Branch: `mvp-vertical-introspection`
- PR title: `cli+gateway: vertical list/describe + GET /verticals[/<name>] (mvp-vertical-introspection)`
