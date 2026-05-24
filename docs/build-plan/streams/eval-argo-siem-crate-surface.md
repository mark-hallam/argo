# eval-argo-siem-crate-surface

**Owner**: claude
**Priority**: P2 - SIEM substrate audit
**Authored against**: origin/main `2860f85e1` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-siem-crate-surface`.

## Source

The mission-vs-code gap manifest still marks `argo-siem` as `NOT AUDITED`. The crate's docs say it tails `audit_record`, batches rows, dispatches to Splunk HEC, Elasticsearch bulk, or Datadog Logs, and persists high-water cursors.

## Goal

Audit and pin the `argo-siem` public surface so SIEM event shape, sink payload formats, dispatcher cursor semantics, and retry/default-off behavior cannot drift silently.

## May Touch

- `crates/argo-siem/tests/surface_contract.rs` (NEW)
- `crates/argo-siem/tests/` (NEW sibling tests if needed)
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-siem-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change production `argo-siem/src/**` code unless a contract test exposes an obvious compile-only drift and the fix is tiny.
- Do not start a live Splunk, Elasticsearch, or Datadog service.
- Do not edit `argo-store` migrations in this audit brief.

## Acceptance

- [ ] Pin exported SIEM types, sink traits, and dispatcher entrypoints from `crates/argo-siem/src/lib.rs`.
- [ ] Cover `SiemEvent` serde/payload shape including tenant/session/audit identifiers if present.
- [ ] Cover Splunk, Elasticsearch, and Datadog sink request-body formatting without network calls.
- [ ] Cover dispatcher config defaults, especially default-off behavior if exposed.
- [ ] Cover cursor/high-water behavior with an in-memory or fake sink fixture if the current API allows it; otherwise document the missing test seam as a finding.
- [ ] Write an eval report with verdict, gaps found, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-siem --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

Prefer fake sinks and pure format tests. If the dispatcher cannot be tested without store/network coupling, record that as the audit result and queue a narrow seam brief.
