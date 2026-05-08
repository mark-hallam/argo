# claude-skill-export-import-bundle — skill artifact metadata sidecar

**Owner**: codex. **Depends on**: existing `argo skill export` /
`argo skill import` artifact flow.

**Authored against**: origin/main `ab1fbc40` on 2026-05-08.

## Conductor Rewrite

Rewritten on 2026-05-08 because current main already exports portable
`.skill.tar.gz` artifacts from `crates/argo-cli/src/handlers/skill_export.rs`
and imports them from `skill_import.rs`. The original brief's
"single markdown file" / new `--bundle` premise is stale. Do not add a
new `--bundle` flag. Extend the existing artifact format with metadata
while preserving import compatibility with older three-file artifacts.

## Why

Mission claim 4 needs reusable shared skills with provenance and trust.
The current artifact already carries a manifest, body, and body hash.
It does not carry operator-facing distribution metadata such as trust
grade, vertical, learned session, or exporter version.

## Scope

- Keep the existing commands:
  - `argo skill export <name[:version]> --out <path>`
  - `argo skill import <path>`
- Add optional `metadata.json` at artifact root when exporting.
- Metadata fields:
  - `artifact_format`: stable string, e.g. `"skycot.skill.v2"`
  - `trust_grade`
  - `vertical_id`
  - `learned_in`
  - `exported_at`
  - `exporter_version`
  - `body_sha256`
- Import must accept both old artifacts without `metadata.json` and new
  artifacts with it.
- Tamper validation remains based on the body hash.

## Approach

1. Extend the existing artifact code path rather than adding a parallel
   bundle implementation.
2. `crates/argo-skill/src/artifact.rs`: add a `SkillArtifactMetadata`
   type and optional read/write support for `metadata.json`.
3. `crates/argo-cli/src/handlers/skill_export.rs`: build metadata from
   the selected `Skill` and include it in the tarball.
4. `crates/argo-cli/src/handlers/skill_import.rs`: ignore/validate the
   optional metadata without changing install semantics.
5. Add focused round-trip/tamper tests.

## Files You May Touch

- `crates/argo-skill/src/artifact.rs`
- `crates/argo-skill/src/lib.rs` (only for re-export if needed)
- `crates/argo-cli/src/handlers/skill_export.rs`
- `crates/argo-cli/src/handlers/skill_import.rs`
- `crates/argo-cli/src/handlers/skill.rs` (only if dispatch output needs
  a small metadata JSON field)
- `crates/argo-cli/src/cli_subcommands_skill.rs` (only if help text needs
  clarification; do not add `--bundle`)
- `crates/argo-cli/tests/skill_bundle_roundtrip.rs` (NEW or extend if it
  already exists)

## Files You Must NOT Touch

- `CLAUDE.md`
- Migration files
- `crates/argo-skill/src/publish.rs`
- Marketplace / OCI mock registry surfaces
- Workspace dependency manifests unless a compile error proves an
  existing dependency is not already available

## Acceptance Criteria

- [ ] Exported `.skill.tar.gz` contains `manifest.json`, `skill.md`,
      `sha256.txt`, and `metadata.json`.
- [ ] `metadata.json` includes trust grade, vertical id when present,
      learned session when present, exporter version, and body hash.
- [ ] Import of a new artifact succeeds and verifies the body hash.
- [ ] Import of an old three-file artifact still succeeds.
- [ ] Tampered body still fails with a clear sha mismatch error.
- [ ] No new workspace dependencies.
- [ ] 5+ tests cover new export metadata, old-artifact import,
      round-trip, sha mismatch, and JSON output.
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file is under 500 LOC.

## Branch + PR

- Branch: `codex-claude-skill-export-import-bundle`
- PR title: `skill: add metadata sidecar to portable artifacts (claude-skill-export-import-bundle)`
