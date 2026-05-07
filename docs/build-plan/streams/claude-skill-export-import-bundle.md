# claude-skill-export-import-bundle — portable skill bundles

**Owner**: claude. **Depends on**: existing `argo skill export` / `argo skill
import` (H-skill-export-import); existing `SkillArtifact*` types in
`crates/argo-skill/src/artifact.rs`.

## Why

Mission claim 4: "Cross-tenant sharing with provenance + trust." The current
export emits a single markdown file. That's enough for round-tripping inside
one tenant but loses the metadata an outside consumer needs (manifest, trust
grade, signing source, dependency hints). A self-describing **bundle** —
manifest + body + side metadata in a single tarball — closes that gap and
gives us a unit we can later sign / publish / mirror.

## Scope

- `argo skill export <id> --bundle <out.tar.gz>` writes a tarball containing:
  - `manifest.toml` (full `SkillFrontmatter` plus version + computed sha256
    of the body).
  - `skill.md` (the body, identical to the existing export format).
  - `metadata.json` with: trust_grade, learned_in, vertical_id (if any),
    operator_feedback summary (if available), exported_at, exporter version.
- `argo skill import <bundle.tar.gz>` reverses the export: validates the
  body sha256 against `manifest.toml`, refuses on mismatch, re-applies the
  skill (insert-or-no-op on duplicate id+version).
- Existing single-file export remains the default behavior; `--bundle` is
  opt-in.

## Approach

1. **NEW** `crates/argo-skill/src/bundle.rs` (~280 LOC):
   - `pub struct SkillBundle { manifest, body, metadata }`.
   - `pub fn write_bundle(skill: &Skill, meta: BundleMetadata, out: &Path) -> ArgoResult<()>`.
   - `pub fn read_bundle(path: &Path) -> ArgoResult<SkillBundle>` (validates sha).
   - Use existing `tar` + `flate2` workspace deps if present; otherwise hand-roll
     a flat zip via `std::io::Cursor` — DO NOT add new deps.
2. **NEW** `crates/argo-cli/src/handlers/skill_bundle.rs` (~250 LOC):
   - `pub async fn export(skills: &SkillStore, args: SkillExportBundleCmd) -> ArgoResult<()>`.
   - `pub async fn import(skills: &SkillStore, args: SkillImportBundleCmd) -> ArgoResult<()>`.
3. Wire CLI: extend `SkillExportCmd`/`SkillImportCmd` with `--bundle <path>`
   so the existing command shape stays. Handler dispatches to bundle path
   when present.

## Files you may touch

- BLANKET CLI ALLOWLIST applies.
- `crates/argo-skill/src/bundle.rs` (NEW, ≤ 320 LOC).
- `crates/argo-skill/src/lib.rs` — narrowly: 1 `pub mod bundle;` line + re-exports.
- `crates/argo-cli/src/handlers/skill_bundle.rs` (NEW, ≤ 280 LOC).
- `crates/argo-cli/src/handlers/skill_export.rs` — narrowly: branch on `--bundle`.
- `crates/argo-cli/src/handlers/skill_import.rs` — narrowly: branch on `--bundle`.
- `crates/argo-cli/src/cli.rs` — narrowly: add `--bundle` to existing variants.
- `crates/argo-cli/tests/skill_bundle_roundtrip.rs` (NEW, ≤ 250 LOC).

## Files you must NOT touch

- Migration files — bundles are filesystem artefacts only.
- `crates/argo-skill/src/publish.rs` — out of scope.
- The mock OCI marketplace.

## Acceptance criteria

- [ ] `argo skill export <id> --bundle <out>` produces a valid bundle file.
- [ ] `argo skill import <bundle>` round-trips byte-for-byte (body) + frontmatter.
- [ ] Tampered body → import fails with clear sha256 mismatch error.
- [ ] Missing `manifest.toml` in bundle → import fails clearly.
- [ ] No new workspace deps.
- [ ] 6+ tests: roundtrip, sha mismatch, missing manifest, dupe import, JSON output, bundle inspector smoke.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every authored file < 500 LOC.

## Branch + PR

- Branch: `claude-skill-export-import-bundle`
- PR title: `skill: portable skill bundle export/import (claude-skill-export-import-bundle)`

## Notes

- Existing `H-skill-export-import.md` is the prior art — read it before starting.
- Keep the bundle layout flat (3 files at root), not nested — simplifies tooling.
- Future work: detached signature file alongside the bundle; not in this brief.
