# V1.2 — Skill marketplace search

**Owner**: codex. **Depends on**: vertical-publish + LayeredLookup
merged (already on main).

## Scope

`argo skill search <query>` searches the OCI skill registry
across tenants. Returns top 20 with TrustGrade, tenant,
description, version, last-published time.

- Hit OCI registry (or mocked manifest index for v1)
- Filter by `--trust-min <grade>`, `--tenant <name>`,
  `--limit <n>`
- Output table by default; `--json` supported (universal)

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-skill-store/src/marketplace.rs` (NEW or
  extend) — narrowly: add `search()` fn
- `crates/argo-cli/src/handlers/skill_search.rs` (NEW)
- `crates/argo-cli/src/cli_skill_cmds.rs` — narrowly:
  `Search` subcommand variant
- `crates/argo-cli/tests/cli_skill_search_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo skill search <query>` returns ranked matches
- [ ] `--trust-min`, `--tenant`, `--limit` filters work
- [ ] `--json` follows universal-json convention
- [ ] 4 unit tests + 1 CLI smoke test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-skill-marketplace-search`
- PR title: `skill: marketplace search across OCI registry (codex-skill-marketplace-search)`
