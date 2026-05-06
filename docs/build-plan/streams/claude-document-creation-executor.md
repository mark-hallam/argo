# V1.2 — Document creation executor

**Owner**: claude. **Depends on**: A-executor-trait merged.

## Scope

New `ExecutorKind::DocumentCreation` for producing structured
documents: PDFs, Markdown reports, slide decks, structured
JSON/CSV. Mission says "any task a human can do on a PC/Mac"
— composing a deliverable doc is one of the most common.

Action payloads: `{ kind: pdf|markdown|slides|csv, content,
template?, output_path }`. Result: success + output path +
size + sha256.

Backends:
- **Markdown** — direct write, no external dep
- **PDF** — markdown → HTML via `pulldown-cmark` → PDF via
  headless Chrome (reuse the build-pdf toolchain we already
  wrote at tick 62 for PROTOCOL_ARCHITECTURE.pdf)
- **Slides** — markdown-with-slide-separators → reveal.js
  HTML → optionally PDF print
- **CSV** — direct write with proper escaping

Sandboxing: writes go to `<worker.fs_root>/outputs/` only
(per claude-filesystem-executor's per-worker root pattern).
Approval-gated by default since outputs are user-visible.

## Files you may touch
- EXHAUSTIVE-MATCH amendment applies (new ExecutorKind variant)
- `crates/argo-executor-document/` (NEW crate)
- `crates/argo-executor-document/Cargo.toml` (NEW)
- `crates/argo-executor-document/src/{lib,executor,markdown,pdf,slides,csv}.rs` (NEW)
- `crates/argo-executor-document/tests/document_smoke.rs` (NEW)
- `crates/argo-cli/src/handlers/worker_actions_executor.rs` —
  narrowly: add `DocumentCreation` arm to ExecutorKind +
  parse_kind logic
- `Cargo.toml` workspace — narrowly: add new member

## Acceptance criteria
- [ ] All 4 formats produce well-formed output
- [ ] Markdown: rendered headers, code, tables
- [ ] PDF: standalone, A4, no external CDN deps
- [ ] Slides: reveal.js horizontal navigation
- [ ] CSV: RFC 4180 compliant escaping
- [ ] Sandboxed writes to fs_root/outputs only
- [ ] Approval-gate marks correct
- [ ] 12 unit tests + 2 integration (PDF + slides)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-document-creation-executor`
- PR title: `runtime: document creation executor (PDF/MD/slides/CSV) (claude-document-creation-executor)`
