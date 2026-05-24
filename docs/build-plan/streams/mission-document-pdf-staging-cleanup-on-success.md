# mission-document-pdf-staging-cleanup-on-success

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `8da732a7d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-document-pdf-staging-cleanup-on-success`.

🟡 **P2 DOCUMENT-EXECUTOR PRIVACY HARDENING.** The document-executor
cross-tenant audit found that PDF rendering writes the markdown-derived HTML to
`<bundle>/.staging/render.html` and leaves it there after a successful render.
That staging file is useful for failed-render post-mortems, but keeping it on
success creates an avoidable plaintext duplicate of the generated document.

## Source

- `docs/skycot/evals/scheduled/2026-05-23/eval-document-executor-cross-tenant-scoping.md`
  finding F-2.
- Current anchor: `crates/argo-executor-document/src/pdf.rs`.

## Scope

- Delete `.staging/render.html` after Chrome PDF generation succeeds.
- Preserve `.staging/render.html` when Chrome invocation fails so operators
  still have failure diagnostics.
- Do not solve the wider tenant-stamped output-path problem here; that is
  already queued as the sibling document-executor tenant/path isolation work.

## May-touch

- `crates/argo-executor-document/src/pdf.rs`
- `crates/argo-executor-document/tests/` — only if integration coverage is
  cleaner than a unit test in `pdf.rs`.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip only.

## Must not touch

- CSV, GSuite, or MS Graph executors.
- Gateway dispatch shims.
- Bundle path shape outside the PDF staging cleanup behavior.

## Acceptance criteria

- [ ] Successful PDF render removes `<bundle>/.staging/render.html`.
- [ ] Failed PDF render leaves `<bundle>/.staging/render.html` available for
      post-mortem.
- [ ] Cleanup failure is surfaced as an executor error or logged in a way that
      cannot silently mislead the caller.
- [ ] Regression coverage pins both success-cleanup and failure-preserves
      behavior; add a small helper seam if direct Chrome invocation makes the
      behavior hard to test.
- [ ] `cargo clippy -p argo-executor-document --all-targets -- -D warnings`
      clean.
- [ ] Every modified file stays under 500 LOC.

