# mission-audit-csv-export — `argo audit export` for operator CSV/JSONL (post-v1 polish P1)

**Owner**: claude. **Depends on**: none (single-crate; reuses existing audit-feed query helpers).
**Authored against**: origin/main `223d3fab` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-csv-export`.
**Source**: post-v1 polish — enterprise procurement check ("can I export audit logs to my SIEM?").

## Mission alignment

- **Observability** (CLAUDE.md priority #2) — audit chain is hash-chained; export gives operators a portable view for SIEM ingestion (Splunk, Datadog, Sumo).
- **Commercial usability** (#5) — enterprise diligence asks for this exact feature.
- **Repeatability** (#3) — deterministic ordering by audit row sequence number.

## Scope

P1. New CLI subcommand:
```bash
argo audit export --format csv|jsonl|json [--since 7d] [--tenant <id>] [--kind ToolDispatch,ActionExecuted] [--out <path>]
```

Behavior:
- **`--format csv`** — RFC 4180 CSV with header row: `seq,session_id,tenant_id,worker_id,timestamp,kind,parent_hash,self_hash,payload_json`. Payload JSON column is the canonical-form payload (sorted keys, no whitespace) — same as what feeds the chain hash.
- **`--format jsonl`** — one JSON object per line; same fields.
- **`--format json`** — single top-level array.
- **`--since <duration>`** — accepts `7d`, `24h`, `30m`, etc. Mutually exclusive with `--from <iso8601>` / `--to <iso8601>` (follow-up).
- **`--tenant <id>`** — restrict to one tenant; defaults to all tenants the principal can read (RBAC-gated).
- **`--kind <comma-list>`** — restrict to specific AuditKind variants.
- **`--out <path>`** — write to file; defaults to stdout.

RBAC: `Capability::AuditRead { scope: AuditReadScope }` — caller must already have it. Cross-tenant export requires admin role.

Streaming: do not materialize the full result in memory. Stream rows from the SQLite query to writer in batches (e.g. 1000-row chunks).

LOC: ~250.

## Files you may touch

- `crates/argo-cli/src/cli_audit_cmds.rs` — extend with `Export` variant. ≤15 modified LOC.
- `crates/argo-cli/src/handlers/audit_export.rs` (NEW) — handler. ≤140 LOC.
- `crates/argo-cli/src/handlers/audit_export_tests.rs` (NEW) — fixture-driven tests (golden CSV + JSONL output). ≤120 LOC.
- `crates/argo-cli/src/handlers/mod.rs` — register handler. ≤3 modified LOC.
- `crates/argo-audit/src/csv.rs` (NEW) — pure formatting helper: `write_audit_csv_row(row, writer)`. ≤60 LOC.
- `crates/argo-audit/src/lib.rs` — re-export. ≤3 modified LOC.
- `crates/argo-audit/src/csv_tests.rs` (NEW). ≤80 LOC.
- `docs/skycot/cli-reference.md` — add the `audit export` row. ≤10 modified LOC.
- `docs/skycot/operator/audit-export.md` (NEW) — operator guide: SIEM ingestion examples for Splunk/Datadog/Sumo. ≤60 LOC.

## Files you must NOT touch

- `argo-store` query layer — reuse existing audit-feed query functions. Add a new public query function if needed but keep schema unchanged.
- The audit chain hash logic — export is read-only.
- `routes_audit_feed.rs` (gateway) — that's the HTTP surface; CLI export is offline DB read.

## Acceptance criteria

- [ ] `argo audit export --format csv --since 1h` writes a valid CSV with the header row above.
- [ ] `argo audit export --format jsonl --tenant ws-default` writes one JSON object per line, filtered to that tenant.
- [ ] `argo audit export --format json` writes a top-level array.
- [ ] `argo audit export --format csv --kind ToolDispatch,ActionExecuted` filters to only those kinds.
- [ ] `argo audit export --format csv --out /tmp/x.csv` writes to file, prints nothing to stdout.
- [ ] Memory usage stays bounded for a 100k-row export (streaming, not buffered).
- [ ] CSV passes a CSV parser round-trip (no quoting/escaping bugs on payloads containing commas, quotes, newlines).
- [ ] RBAC enforced: caller without `AuditRead{cross_tenant}` and no `--tenant <self>` flag → returns error.
- [ ] `cargo test -p argo-audit -p argo-cli` passes.
- [ ] `cargo clippy -p argo-audit -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-audit-csv-export`
- Title: `feat(cli): argo audit export --format csv|jsonl|json (mission-audit-csv-export)`
- Worker verification: crate-scoped clippy + tests + `cargo clippy --workspace` per SWARM_PROTOCOL.md.

## Hidden constraints

- **CSV escaping is subtle.** Use `csv` crate (already in Cargo workspace via argo-store) — don't hand-roll. RFC 4180 quoting handles commas, quotes (doubled), and CRLF in fields.
- **Canonical payload JSON** — match the form used by the chain hash (`canonical_payload_json` per `crates/argo-audit/src/sink.rs`). Sorted keys, no whitespace. Diverging from this breaks the operator's ability to recompute hashes from the export.
- **Streaming writer** — `BufWriter<File>` or `BufWriter<Stdout>`. Flush at end.
- **`--since` parsing** — humantime crate or argo-config's existing duration parser; pick whichever is already in argo-cli deps.
- **Don't include sealed/elevated payloads in plain export.** If a row's `payload_json` contains sealed fields (per any pre-existing seal logic), export them as `"[sealed]"` not the cleartext.
