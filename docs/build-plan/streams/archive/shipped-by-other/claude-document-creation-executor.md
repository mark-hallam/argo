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
- [x] All 4 formats produce well-formed output
- [x] Markdown: rendered headers, code, tables — pulldown-cmark stays
      faithful to source markdown; verified by smoke test
- [x] PDF: standalone, A4, no external CDN deps — print stylesheet
      bundled inline; Chrome via `ARGO_DOC_CHROME` (default macOS path)
- [x] Slides: reveal.js horizontal navigation — embedded keyboard-nav
      script (no CDN); arrow keys / space / Home / End cycle slides
- [x] CSV: RFC 4180 compliant escaping — quote on `, " \r \n`; double
      internal quotes; CRLF row terminators; row-arity guard
- [x] Sandboxed writes to fs_root/outputs only — `OutputPath::resolve`
      rejects absolute paths + parent traversal + empty components
- [x] Approval-gate marks correct — DocumentCreation `ExecutorKind`
      flows through the existing AutoApprovalPolicy gate (which keys
      off `requires_approval` from manifest); approval-gated by default
      since outputs are user-visible
- [x] 12 unit tests + 2 integration (PDF + slides) — 24 unit (2 md +
      6 csv + 4 pdf + 4 slides + 8 executor) + 5 integration (markdown
      + csv + slides + pdf-missing + path-escape) shipped
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-document-creation-executor`
- PR title: `runtime: document creation executor (PDF/MD/slides/CSV) (claude-document-creation-executor)`
