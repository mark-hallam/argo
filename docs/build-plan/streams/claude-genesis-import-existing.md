# Genesis import existing — convert prompt-repo into vertical scaffold

**Owner**: claude. **Depends on**: none.

## Scope

Operators frequently arrive with an existing system prompt: a
LangChain prompt template in a Python repo, a markdown file in a
github prompt-engineering repo, or a chunk of text from a Notion doc.
This brief adds `argo genesis import` that reads such a source and
produces a vertical scaffold (`vertical.toml`, `system-prompt.md`,
empty `eval-seed.toml`, empty `memory-schema.toml`).

Genesis's normal generators (action-suggestor, eval-seed-generator,
memory-schema-builder) can then refine the scaffold via `genesis revise`.

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/genesis_import.rs` (~280 LOC):
   - `pub async fn handle_import(args: GenesisImportArgs) -> ArgoResult<()>`
   - Args: `--source <path|url>`, `--name <vertical-name>`,
     `--format <auto|langchain|markdown|raw>` (default: auto),
     `--output <verticals/dir>` (default: `verticals/`)
   - Auto-detect format:
     - `*.py` containing `PromptTemplate(` → langchain
     - `*.md` → markdown (extract first H1 + first code block as prompt)
     - `*.txt` / raw → use full file as prompt verbatim
   - Generates a minimal `vertical.toml` with the name, prompt path,
     and empty action/eval/memory sections (Genesis revise can fill
     them later)
2. **Modify** `argo-cli/src/cli.rs`: add `Genesis::Import { ... }` subcommand
3. **Modify** `handlers/mod.rs`: 1 pub mod line

## Files you may touch

- `crates/argo-cli/src/handlers/genesis_import.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/genesis_import_tests.rs` (NEW, ≤ 220 LOC) — 6 tests:
  - import a sample langchain `*.py` → extracts the PromptTemplate string
  - import a markdown file → extracts H1 as title, body as prompt
  - import raw text → uses verbatim
  - auto-detect picks the right format for each
  - missing source path → ArgoError::NotFound
  - existing vertical name → ArgoError::Validation (no overwrite)
- `crates/argo-cli/tests/fixtures/import_langchain_sample.py` (NEW, ≤ 30 LOC) — minimal langchain-style file
- `crates/argo-cli/tests/fixtures/import_markdown_sample.md` (NEW, ≤ 20 LOC)
- `crates/argo-cli/src/cli.rs` (narrowly: add Import subcommand)
- `crates/argo-cli/src/handlers/mod.rs` (narrowly: 1 pub mod line)
- `crates/argo-cli/tests/genesis_import_smoke.rs` (NEW, ≤ 180 LOC) —
  end-to-end: import a fixture markdown into a tempdir, assert
  vertical.toml + system-prompt.md exist with non-empty content

## Files you must NOT touch

- Network: NO HTTP fetches in v0 — `--source` is a local path only.
  URL support belongs in a follow-up (security review needed).
- `crates/argo-genesis/` — import is a CLI-only feature; the scaffold
  it produces is consumed by existing genesis modules later

## Acceptance criteria

- [ ] `argo genesis import --source fixture.md --name imported-test` writes a valid vertical
- [ ] Auto-detect picks langchain / markdown / raw correctly for the 3 fixtures
- [ ] Refuses to overwrite an existing vertical name
- [ ] Empty action/eval/memory sections are well-formed TOML (parse via toml::from_str)
- [ ] 6 unit tests + 1 smoke test pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-genesis-import-existing`
- PR title: `genesis: import langchain/markdown/raw prompt into vertical scaffold (claude-genesis-import-existing)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies — cli.rs / handlers/mod.rs / Cargo.toml
  edits are pre-approved.
- Local-only `--source` removes the URL-fetch security surface for v0.
  Document the deferral with a TODO comment.
- For langchain extraction, regex `PromptTemplate\(\s*template\s*=\s*"""(.+?)"""` (DOTALL)
  is enough for v0; LangChain Python AST parsing is overkill.
