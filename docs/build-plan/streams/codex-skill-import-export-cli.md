# V2.0 — Skill import/export CLI

**Owner**: codex. **Depends on**: nothing (skill core already
merged).

## Scope

`argo skill search` (#849) and `argo skill publish`
(codex-skill-marketplace-publish-cli, queued) cover online
distribution. Missing: offline import/export so an operator
can move skills between workspaces / hand-deliver via tarball
when the marketplace isn't reachable.

Add:
- `argo skill export <name>[:<version>] --out <path> [--json]`
  — packages a single skill (manifest + body + signed metadata)
  into a `.skill.tar.gz` artifact.
- `argo skill export-all --out <dir> [--vertical <id>] [--json]`
  — batch-exports every skill in workspace (or vertical-filtered)
  into a directory.
- `argo skill import <path> [--name <new-name>] [--workspace
  <path>] [--dry-run] [--json]` — installs a `.skill.tar.gz`
  into the local skill store, applies signature check, refuses
  duplicate-name unless `--name` rename provided.

Mirror pattern: read existing `crates/argo-cli/src/handlers/
skill_export.rs` and `skill_import.rs` (which exist as stubs
per `ls`), wire the actual tarball logic.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/skill_export.rs` —
  narrowly: implement actual tarball pack
- `crates/argo-cli/src/handlers/skill_import.rs` —
  narrowly: implement actual tarball unpack
- `crates/argo-cli/src/handlers/skill_export_all.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod
- `crates/argo-cli/src/cli_skill_io_cmds.rs` (NEW) — or
  extend existing skill subcommand enum if cleaner
- `crates/argo-skill/src/artifact.rs` (NEW) — pure tarball
  pack/unpack + signature verify
- `crates/argo-skill/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/tests/skill_io_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo skill export foo --out /tmp/foo.skill.tar.gz` —
      tarball lands at path with manifest + body + sha256
- [ ] `argo skill import /tmp/foo.skill.tar.gz` — skill lands
      in local store; subsequent `argo skill list` shows it
- [ ] `--dry-run` on import prints what would land, no side
      effects
- [ ] `--name <newname>` rename works for duplicate-name
      conflict
- [ ] Round-trip: export → wipe workspace skill_store → import
      → identical skill
- [ ] `--json` envelope universal across all 3 subcommands
- [ ] 6 unit tests + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-skill-import-export-cli`
- PR title: `skill: argo skill import/export — offline tarball pack/unpack (codex-skill-import-export-cli)`
