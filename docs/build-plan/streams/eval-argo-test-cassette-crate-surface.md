# eval-argo-test-cassette-crate-surface

**Owner**: claude
**Priority**: P3 - test substrate audit
**Authored against**: origin/main `ef55f5422` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-test-cassette-crate-surface`.

## Source

The mission-vs-code gap manifest lists `argo-test-cassette` as test
infrastructure used by the swarm. It is not mission-critical production code,
but it controls record/replay determinism and secret redaction for connector and
integration tests.

## Goal

Pin the cassette crate's public contract so future connector work does not
silently weaken replay determinism, JSON compatibility, or secret redaction.

## May Touch

- `crates/argo-test-cassette/tests/surface_contract.rs` (NEW)
- `crates/argo-test-cassette/src/cassette_tests.rs` (only if extending existing unit coverage is cleaner)
- `crates/argo-test-cassette/fixtures/` (new minimal fixture only if needed)
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-test-cassette-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change connector crates or production code.
- Do not add live network calls.
- Do not change cassette JSON version unless the report explicitly justifies a follow-up.

## Acceptance

- [ ] Pin the exported inventory from `crates/argo-test-cassette/src/lib.rs`.
- [ ] Cover cassette JSON load/save compatibility using a small fixture.
- [ ] Cover header and body redaction for authorization/token/secret-like values.
- [ ] Cover replay/default mode behavior without network access.
- [ ] Cover record construction shape for method, URL, headers, body hash, and response status.
- [ ] Write an eval report with verdict, any compatibility risk, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-test-cassette` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

This is an audit/pin brief. Prefer tests and report over production rewrites.
