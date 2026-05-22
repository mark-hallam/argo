# V2.0 — Skill marketplace publish CLI

**Owner**: codex. **Depends on**: codex-skill-marketplace-search
(#849) merged.

## Scope

`#849` shipped `argo skill search` — discovering skills across
the OCI registry. The companion publish path is missing: a tenant
that develops a useful skill cannot share it back to the
marketplace.

Add:
- `argo skill publish <name> [--registry <ref>] [--tag <tag>]
  [--manifest <path>] [--dry-run] [--json]` — packages a skill
  into an OCI artifact and pushes to the configured registry.
- `argo skill unpublish <name>:<tag> [--registry <ref>] [--json]`
  — soft-delete (tag remove only; underlying blob preserved).
- `argo skill versions <name> [--registry <ref>] [--limit N]
  [--json]` — list all published tags for a given skill name.

Re-uses the manifest format already defined for skill search.
Auth via existing credentials.toml (`OCI_REGISTRY_USER` /
`OCI_REGISTRY_PASS`) with --registry flag override.

Mock-friendly: --registry mock://path drops to a local filesystem
"registry" so smoke tests don't depend on network. Mirrors how
#849 mocks the index.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/skill_publish.rs` (NEW)
- `crates/argo-cli/src/handlers/skill_unpublish.rs` (NEW)
- `crates/argo-cli/src/handlers/skill_versions.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 pub mods
- `crates/argo-cli/src/cli_skill_publish_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_skill_search_cmds.rs` — narrowly:
  add `Publish`/`Unpublish`/`Versions` variants if `Skill`
  enum lives there; otherwise create siblings
- `crates/argo-skill/src/publish.rs` (NEW) — pure logic
  (manifest assembly + OCI artifact tarball)
- `crates/argo-skill/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/tests/skill_publish_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo skill publish my-skill --registry mock://t` writes
      tarball + manifest to t/oci/my-skill/<tag>
- [ ] `argo skill versions my-skill --registry mock://t` lists
      all tags published
- [ ] `argo skill unpublish my-skill:0.1.0 --registry mock://t`
      removes the tag (file/dir gone) but keeps the underlying
      blob (preserved for forensics)
- [ ] `--dry-run` prints what would be published, makes no
      network calls
- [ ] `--json` envelope universal across all 3 subcommands
- [ ] 5 unit tests + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-skill-marketplace-publish-cli`
- PR title: `skill: argo skill publish — OCI marketplace push (codex-skill-marketplace-publish-cli)`
