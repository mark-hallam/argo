# AuditSink construction policy

`AuditSink::new(store)` is `#[deprecated]`. Production callers must
use `AuditSink::new_with_tenant(store, tenant_id)` so every audit row
carries the originating tenant — cross-tenant audit drift is a
multi-tenant invariant violation. The guard is enforced by
`scripts/lint-no-audit-sink-new-in-production.sh` under the
`audit-sink-tenant-guard` CI job. Golden tests:
`scripts/lint-no-audit-sink-new-in-production_tests.sh`.

## Skip rules

- Paths ending in `_test_helpers.rs`, `_tests.rs`, `tests.rs`.
- Anything inside a `tests/` or `benches/` directory.
- Files opening with the inner attribute `#![allow(deprecated)]` —
  file-level opt-in only; per-function `#[allow(deprecated)]` does
  NOT count.
- `//` comments and lines inside `#[cfg(test)] mod tests { … }`.

## Adding a documented exception

Prefer, in order:

1. Migrate to `AuditSink::new_with_tenant` — pipe the request scope's
   `tenant_id` through.
2. Rename / move the helper into the test-sibling convention
   (`_test_helpers.rs` / `_tests.rs`) if only reachable from tests.
3. Last resort: add `#![allow(deprecated)]` at the top of the file
   with a `//!` doc comment naming the constraint, AND open a
   follow-up brief to remove the exception.

Do not extend the lint with per-path allowlists; the skip rules above
are intentionally generic so the policy stays grep-able. Closes
2026-05-17 audit finding #9.
