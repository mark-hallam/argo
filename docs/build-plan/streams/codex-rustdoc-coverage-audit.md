# codex-rustdoc-coverage-audit

**Authored against**: `origin/main` `b566866d4` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-rustdoc-coverage-audit`.

**Fast-mode eligible**: yes — pure docs audit + small fills.

## Summary

[CLAUDE.md](CLAUDE.md) invariant: "Every public Rust API has rustdoc with at least one example". Audit the workspace for compliance and fill ≤10 of the worst offenders.

## Acceptance

- [ ] Enumerate every `pub fn`, `pub struct`, `pub enum`, `pub trait` across `crates/*/src/`.
- [ ] Classify each:
  - **DOCUMENTED-WITH-EXAMPLE** — has `///` block + at least one `# Examples` section or `# Example` heading with code.
  - **DOCUMENTED-NO-EXAMPLE** — has `///` block but no example.
  - **UNDOCUMENTED** — no `///` block.
  - **DOC-COMMENT-NOT-NEEDED** — internal-helper-flavored type re-exported because of macro constraints; safe to skip.
- [ ] PR body markdown table: crate | symbol | classification.
- [ ] For ≤10 of the highest-traffic UNDOCUMENTED or DOCUMENTED-NO-EXAMPLE entries (judge by callsite count via `rg -c`), add a `///` block with a minimal compilable example.
- [ ] Examples must compile cleanly (`cargo doc --no-deps` + `cargo test --doc`). Don't add `no_run` unless genuinely required.
- [ ] No semantic change.

## May touch

- `crates/*/src/**/*.rs` — only the doc-comment additions on the ≤10 chosen entries.

## Must NOT touch

- Production code logic.
- Test files.
- Re-exports / module structure.

## Notes

- Pre-flight: `rg -c 'pub fn ' crates/argo-store/src/ crates/argo-types/src/ crates/argo-audit/src/ crates/argo-skill/src/` to pick the highest-density-of-undocumented-pubs crate.
- Sizing: 200-400 LOC (mostly doc comments). 10 entries × ~25 LOC each.
- Procurement-grade leverage — buyers reviewing the codebase check rustdoc coverage.
