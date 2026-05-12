# Mission load-bearing wire — Audit-kind runtime event variants

**Owner**: claude. **Depends on**: `mission-context-window-aware-compression` MERGED (#1465); `mission-retro-review-ui` MERGED (#1480).
**Authored against**: origin/main `77ee9a59` on `2026-05-13`.

> SOURCE: PR-deferral from #1465 + #1480. Both shipped audit-row-emission patterns that fall back to `AuditKind::HttpRequest` with an `event_type` payload field because `argo-audit` was outside their may-touch lists. This brief adds the proper enum variants so the audit chain validators can categorize correctly.

## Scope

Add new variants to `argo_audit::AuditKind`:

1. `RuntimeContextCompressed` — emitted by `context_compress::build_audit_payload`
2. `RetroCreated` / `RetroFinalized` — emitted by retro lifecycle
3. `SecurityKeyRotated` — promote from `HttpRequest+event_type` to first-class
4. `ApprovalExpiredByTimeout` — promote from `HttpRequest+event_type` to first-class

For each variant:
- Add to the `AuditKind` enum
- Add to the `label()` impl (mapping to canonical string label used in the chain)
- Add to `from_label_lossy()` / parser
- Update existing emit sites (4 of them) to use the new variant instead of `HttpRequest+event_type` workaround

Hash chain unchanged — the chain hashes `kind.label()` which is a string. The change is internal-strongly-typed-only; existing audit rows on main keep working (their `HttpRequest` rows still parse).

NOT in scope: deleting the `event_type` payload field (legacy rows keep using it); adding ApprovalBundle audit kinds (separate brief); migrating historical audit rows (chain integrity).

## Files you may touch

- `crates/argo-audit/src/event.rs` or wherever `AuditKind` lives (≤ 280 LOC after) — 4 new variants + label mapping
- `crates/argo-audit/src/event_tests.rs` (NEW or extend, ≤ 200 LOC) — 4 tests: each variant round-trips through `label()` ↔ `from_label_lossy()`
- 4 emit sites:
  - `crates/argo-runtime/src/context_compress.rs` — switch from `HttpRequest+event_type` to `RuntimeContextCompressed`
  - `crates/argo-cli/src/handlers/gateway/routes_retros.rs` — `RetroCreated` + `RetroFinalized`
  - `crates/argo-cli/src/handlers/security_rotate_keys.rs` — `SecurityKeyRotated`
  - `crates/argo-cli/src/handlers/gateway/approval_expiry_sweeper.rs` (if it exists; else flag missing) — `ApprovalExpiredByTimeout`

## Files you must NOT touch

- Hash chain logic (chain takes `kind.label()` as string; new variants get new labels, but historical chains stay valid because their on-disk strings are unchanged)
- Audit row reader / verifier (they parse strings; new strings are fine)

## Acceptance criteria

- [ ] 4 new `AuditKind` variants compile
- [ ] Each variant has a canonical `label()` string + roundtrips through parser
- [ ] 4 emit sites updated to use new variants
- [ ] Existing audit rows on main parse unchanged (`HttpRequest` rows still valid)
- [ ] Hash chain unchanged (verified by re-hashing 1+ historical rows in tests)
- [ ] 4 unit tests + 1 integration test (emit + verify chain)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-audit-kind-runtime-events`
- PR title: `audit: promote 4 HttpRequest+event_type patterns to AuditKind variants (mission-audit-kind-runtime-events)`

## Notes

- The migration path matters: existing `HttpRequest` rows with `event_type` payloads remain on disk forever (chain immutable). New emits use the new variant. Audit-feed readers should accept both during a transition period.
- Choose canonical labels carefully — once written to a chain row, they can't change. Recommend: `runtime.context_compressed`, `retro.created`, `retro.finalized`, `security.key_rotated`, `approval.expired_by_timeout`. These match the existing `event_type` payload strings, so audit-feed clients filtering on them keep working.
