# `argo persona edit` — $EDITOR-driven persona update

**Owner**: claude. **Depends on**: —

## Why

Personas evolve. Voice gets refined. Signatures change as the brand
matures. Today the only path is: read the persona JSON, edit by hand,
re-create the persona, re-attach to workers. That's a five-step dance
operators won't do. `argo persona edit <name>` opens $EDITOR on a TOML
view of the persona, validates on save, writes if valid.

## Scope

- `argo persona edit <name>` resolves persona, dumps a TOML snippet
  (`[persona] name=... voice=... signature=...`) to a temp file,
  spawns $EDITOR, on exit re-parses, validates, writes back.
- $EDITOR resolution: `$ARGO_EDITOR > $VISUAL > $EDITOR > "vi"`.
- Validation: name regex, voice ≤ 4 KB, signature ≤ 1 KB.
- If validation fails after save, leave the temp file in place, print
  its path, exit non-zero — do NOT re-prompt.
- Optional `--in-place <toml-path>` to skip $EDITOR (for tests + scripts).

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/persona_edit.rs` (~220 LOC):
   - `pub fn dump_toml(persona: &Persona) -> String`
   - `pub fn parse_toml(s: &str) -> ArgoResult<Persona>`
   - `pub fn resolve_editor() -> String`
   - `pub async fn run(name: &str, in_place: Option<PathBuf>, store) -> ArgoResult<()>`
   - Subprocess shell-out via `std::process::Command`; on tests
     pass `--in-place` instead.
2. **NEW** `crates/argo-cli/src/handlers/persona_edit_tests.rs` (~200 LOC):
   - Round-trip dump→parse equivalence
   - Invalid voice (oversize) → error, no write
   - Invalid signature (oversize) → error, no write
   - Invalid TOML syntax → error, temp path printed
   - --in-place path success
   - --in-place path with file-missing
3. **Modify** `crates/argo-cli/src/cli.rs` — 1 new
   `Persona::Edit { name: String, in_place: Option<PathBuf> }`
   variant. EXHAUSTIVE-MATCH note: handlers/mod.rs dispatch arm.
4. **NEW** `crates/argo-cli/tests/persona_edit_smoke.rs` (~120 LOC) —
   seed persona, run with --in-place pointing at a fixture, assert update.

## Files you may touch

- `crates/argo-cli/src/handlers/persona_edit.rs` (NEW, ≤ 280 LOC)
- `crates/argo-cli/src/handlers/persona_edit_tests.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (1 pub mod + dispatch arm)
- `crates/argo-cli/src/cli.rs` (BLANKET CLI ALLOWLIST — cli.rs is 49 LOC post-#968 split, plenty of headroom)
- `crates/argo-cli/src/cli_command_enum.rs` (post-#968: top-level `Command` variants live here — add `Persona` here)
- `crates/argo-cli/tests/persona_edit_smoke.rs` (NEW)
- `crates/argo-cli/tests/fixtures/persona_edit_valid.toml` (NEW) — small fixture

## Files you must NOT touch

- `crates/argo-identity/` — Persona struct unchanged; only re-use it
- `argo-store/` — re-use the existing persona repo for read+write
- Other `argo persona` subcommands — leave list/show/create alone

## Acceptance criteria

- [ ] `argo persona edit foo --in-place valid.toml` updates persona on disk
- [ ] `argo persona edit foo --in-place oversized.toml` errors, no write
- [ ] Round-trip: dump → parse → equal Persona
- [ ] $EDITOR resolution: `$ARGO_EDITOR` wins over `$VISUAL` over `$EDITOR` over `vi`
- [ ] Invalid TOML → exit 1, temp file path printed, persona unchanged
- [ ] Voice > 4 KB → rejected before write
- [ ] Signature > 1 KB → rejected before write
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-persona-edit-cli`
- PR title: `persona: $EDITOR-driven `argo persona edit` (claude-persona-edit-cli)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies.
- The TOML format is part of the public surface — pin it with
  `#[serde(rename_all = "snake_case")]` and explicit field names so
  we don't break operator scripts later.
- Don't get clever with $EDITOR. Synchronous spawn → wait → read.
  No async, no signal-handling beyond the default.
- `toml = "0.8"` is a workspace dep — verify before adding.
