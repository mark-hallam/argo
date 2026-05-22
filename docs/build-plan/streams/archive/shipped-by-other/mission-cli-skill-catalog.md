# Mission load-bearing wire — `argo skill catalog` browse remote skill marketplace

**Owner**: claude. **Depends on**: `argo-skill::marketplace::MANIFEST_INDEX` exists (verified at `crates/argo-skill/src/marketplace.rs:211`).
**Authored against**: origin/main `d53a8f71` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cli-skill-catalog`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 item #75 ("Build `argo skill catalog` for browsing remote skills"). Today the CLI exposes `skill list` (local installed) but no way to browse what's *available* in the marketplace — operators have to read the source `MANIFEST_INDEX` to know what they could install. Mission Claim 4 ("skills shared and learned") promises a marketplace; the catalog command is the discoverability surface.

## Scope

Add `argo skill catalog [--vertical <id>] [--tag <tag>] [--installed-only | --not-installed | --all] [--json]` CLI subcommand. The default mode walks `MANIFEST_INDEX`, joins each entry against the local installed-skill set, and prints a table:

```
 STATUS   ID                                  NAME                       VERTICAL
 [x]      sales-sdr-cold-outreach             Cold Outreach              sales-sdr
 [ ]      cs-tier1-greeting                   Tier-1 Greeting            customer-support
```

Filters compose: `--vertical recruiter --not-installed` shows only recruiter-tagged uninstalled skills. `--json` emits a structured envelope `[{ id, name, vertical, tags, installed: bool, version }]` for scripting.

This is NOT in scope: actually fetching skill bytes from a remote registry (separate brief #71); install/upgrade flows (those exist as `skill install` / `skill update`); skill ratings UI (#80); cross-tenant catalog (single tenant only for v1).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/skill_catalog.rs` (NEW, ≤ 300 LOC) — argument-parsing + filter pipeline + table renderer + JSON renderer.
- `crates/argo-cli/src/cli_subcommands_skill.rs` — narrowly scoped: ADD a `Catalog { vertical: Option<String>, tag: Option<String>, installed_only: bool, not_installed: bool, all: bool, json: bool }` variant to the existing `SkillCmd` enum.
- `crates/argo-cli/src/handlers/mod.rs` — narrowly scoped: ONE `pub(crate) mod skill_catalog;` line.
- `crates/argo-cli/src/main.rs` — narrowly scoped: ONE arm in skill-dispatch match for `Catalog` calling the new handler.
- `crates/argo-cli/src/handlers/skill_catalog_tests.rs` (NEW, sibling) — 7 unit tests: empty-marketplace; entry-marked-installed-when-local; vertical-filter-applies; tag-filter-applies; installed-only-filters-out-not-installed; not-installed-filters-out-installed; json-output-shape-stable.
- `crates/argo-cli/tests/skill_catalog_smoke.rs` (NEW) — integration smoke: seed local with 1 of N marketplace skills installed; invoke `argo skill catalog`; assert table shows N rows with correct status marks.
- `crates/argo-skill/src/marketplace.rs` — narrowly scoped: ADD `pub fn iter_index() -> impl Iterator<Item = &'static IndexedSkill>` if the existing API doesn't already expose iteration. NO change to `MANIFEST_INDEX` itself.

## Files you must NOT touch

- `crates/argo-skill/src/marketplace.rs` beyond the iter_index addition — `MANIFEST_INDEX` shape is upstream contract
- `crates/argo-store/migrations/**` — no schema change
- Any other skill handler beyond the new module
- `crates/argo-tool/**` — orthogonal

## Acceptance criteria

- [ ] `argo skill catalog` (no flags) prints table of all marketplace skills with status marks
- [ ] `--vertical <id>` filters to that vertical's skills only
- [ ] `--tag <tag>` filters to skills carrying that tag (substring match against the tag list)
- [ ] `--installed-only` shows ONLY locally-installed entries
- [ ] `--not-installed` shows ONLY uninstalled entries
- [ ] `--all` is the default (no filter)
- [ ] `--installed-only` + `--not-installed` together is a usage error (clap-level if possible)
- [ ] `--json` emits a JSON array `[{ id, name, vertical, tags, installed, version }]` to stdout instead of the table
- [ ] Status mark format: `[x]` installed, `[ ]` not installed (3-character-wide column)
- [ ] When marketplace is empty, prints `no skills in catalog` and exits 0
- [ ] 7 new unit tests cover the filter / format matrix
- [ ] 1 integration smoke exercises end-to-end
- [ ] `cargo build -p argo-cli -p argo-skill` clean
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cli-skill-catalog`
- PR title: `cli: argo skill catalog with vertical/tag filters + installed-status (mission-cli-skill-catalog)`

## Notes for the implementer

- The local installed-skill set: there's an existing API (`SkillStore::list_installed` or similar — verify in first 5 minutes via `rg -n 'list_installed\|installed_skills' crates/argo-skill crates/argo-store`). Use that for the join.
- Table renderer: there's likely an existing helper in `argo-cli` for column-aligned text output (look at `skill_list.rs` for the pattern). Reuse that — don't introduce a new table crate.
- `IndexedSkill` shape (verify): probably `{ id, name, vertical, tags: &[&str], version }`. Map cleanly into the JSON envelope.
- The "version" column is the marketplace version, which may differ from the installed version. If both are present, include `installed_version` in JSON; in the table, just show the marketplace version (operators rarely care about the diff at browse time).
- Don't add network calls. The marketplace today is static (`const &[IndexedSkill]`). Remote-fetch is brief #71 (`Replace hardcoded MANIFEST_INDEX with real OCI registry catalog scan`); keep this brief separate.
- If `--vertical` is supplied with an unknown id, print `vertical <id> not found` and exit 1 (don't print empty table — that's confusing).
