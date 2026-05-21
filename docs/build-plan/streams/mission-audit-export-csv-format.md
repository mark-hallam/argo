# mission-audit-export-csv-format

**Authored against**: `origin/main` `32631c0ae` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-export-csv-format`.

## Summary

The existing `argo audit export` (sibling to #1921's tar bundle with `verify.sh`) produces JSON. Procurement reviewers and compliance teams routinely ask for CSV — spreadsheet-friendly, easy to grep, supported by every audit-trail review tool. Add CSV as a sibling format option without removing JSON.

## Acceptance

- [ ] `argo audit export --format csv` produces a CSV with header row + one row per audit event.
- [ ] CSV column order: `timestamp_utc, tenant_id, principal_id, kind, target_kind, target_id, decision, payload_summary` (verify canonical field set in `crates/argo-types/src/audit.rs`).
- [ ] `payload_summary` is a short 1-line stringification of the JSON payload (truncate at 200 chars with "…" if longer).
- [ ] CSV escaping correct: fields containing comma / newline / double-quote get RFC 4180 double-quote wrapping.
- [ ] `--format json` (existing default) unchanged.
- [ ] `--format jsonl` if it exists today is also unchanged.
- [ ] Unit test: feed a fixture audit event with comma + newline + quote → assert correct escaping in CSV output.
- [ ] Integration test: `argo audit export --format csv --to /tmp/out.csv` writes a parseable CSV that `csv.reader` (Python) round-trips.
- [ ] Tar bundle from #1921 gets a sibling CSV alongside the JSONL (no breaking change — the verify.sh script still validates JSONL chain).

## May touch

- `crates/argo-cli/src/handlers/audit_export.rs` (or wherever `audit export` is handled)
- `crates/argo-audit/src/export.rs` (new file or extension of existing exporter)
- Test files
- CLI help text for the `--format` flag

## Must NOT touch

- The JSONL bundle format (#1921 settled).
- The `verify.sh` script that ships in the tar bundle (#1921 settled).
- The audit chain integrity logic.

## Notes

- Pre-flight: `rg -n 'audit_export\|--format' crates/argo-cli/` to find current export shape.
- Sizing: 150–300 LOC. Single new format option + tests.
- Procurement-grade leverage; ties into mission claim 6.
