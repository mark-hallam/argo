# codex-audit-kind-emit-site-coverage

**Authored against**: `origin/main` `6fda55b5` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-audit-kind-emit-site-coverage`.

**Fast-mode eligible**: yes — pure mechanical audit.

## Summary

49 `AuditKind` variants live in `crates/argo-audit/src/event.rs`. Every variant should be emitted from at least one production callsite (not just enumerated). An unused variant is dead code; a partially-wired variant is theater. Codex audits every variant for emit-site coverage and reports gaps.

## Acceptance

- [ ] Enumerate every variant in `AuditKind`. For each, `rg 'AuditKind::<Variant>' crates/ web/` produces the callsite list.
- [ ] Classify each variant:
  - **OK** — at least 1 production callsite (gateway / runner / CLI handler).
  - **TEST-ONLY** — only appears in `#[cfg(test)]` or `tests/` dirs → flag in report.
  - **NONE** — no callsite anywhere → dead variant → flag for deletion or wire-up.
- [ ] PR body includes a markdown table: variant | status | callsite count | note.
- [ ] For variants in TEST-ONLY or NONE state: 1-paragraph recommendation per variant — wire it up vs delete vs leave-for-follow-up.
- [ ] If a wire-up is ≤5 LOC (call existing AuditSink with already-available context), fix it in this brief.
- [ ] No semantic change to any existing emit site.

## May touch

- Production code where a ≤5-LOC wire-up is genuinely additive (no refactor of the calling function).
- Test fixtures or unit tests if the audit reveals a missing test case.

## Must NOT touch

- The `AuditKind` enum itself (no variant additions/deletions in this brief — fix recommendations only).
- Test-only callsites that are intentional (e.g. integration-test fixtures).
- The audit-chain integrity logic.

## Notes

- Auditor signal: claim 4 + claim 6 procurement story hinges on audit-chain completeness. Theater variants undermine the story.
- Pre-flight: read [crates/argo-audit/src/event.rs](crates/argo-audit/src/event.rs) for the canonical variant list (49 as of `6fda55b5`).
- Sizing: 100–400 LOC total (mostly report + ≤5-LOC wire-ups). If the audit reveals deep gaps (>5 variants need substantial wire-up), produce the report + spawn follow-up briefs rather than scope-creep this one.
