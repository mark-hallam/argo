# Mission load-bearing wire — CSV document creation executor

**Owner**: claude. **Depends on**: existing executor scaffold patterns (verify shape via `ls crates/argo-executor-*/` — pattern is per-executor crate + dispatch arm in argo-cli worker_actions_executor.rs); RFC 4180 CSV-quoting conventions.
**Authored against**: origin/main `694a50ee` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-document-creation-executor-csv`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #105 ("Build document creation executor — CSV (RFC 4180)"). **Tier A (executor).** Several verticals (`data-entry-clerk`, `bookkeeping-assistant`, `procurement-assistant`) need to emit structured tabular output — today they hand back markdown tables which spreadsheet operators have to manually reshape. This brief lands a NEW `argo-executor-document-csv` crate that takes a typed-row stream + column schema + path, produces an RFC 4180–conforming `.csv` artifact, and surfaces it into the worker action bundle.

## Scope

NEW crate `crates/argo-executor-document-csv/` (≤ 700 LOC total):

```rust
// crates/argo-executor-document-csv/src/lib.rs
pub struct DocumentCsv;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvColumnSchema {
    pub header: String,
    pub kind: CsvColumnKind, // Text | Integer | Decimal | Date | Boolean | EmailAddress
    pub required: bool,
    pub max_len: Option<u32>,
}

#[derive(Debug)]
pub struct CsvWriteRequest {
    pub path: PathBuf,            // operator-side target; sandbox enforced
    pub schema: Vec<CsvColumnSchema>,
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    pub include_header: bool,
    pub line_terminator: LineTerminator, // CRLF (default per RFC 4180) | LF
    pub quote_minimal: bool,       // true = quote only when needed; false = quote-all
}

pub struct CsvWriteReport {
    pub path: PathBuf,
    pub bytes_written: u64,
    pub rows_written: u32,
    pub rows_rejected: u32,
    pub rejections: Vec<RowRejection>, // missing_required / type_mismatch / max_len_exceeded
    pub sha256: [u8; 32],          // of the resulting file
}

impl DocumentCsv {
    pub fn new() -> Self;
    pub async fn write(&self, req: CsvWriteRequest) -> ArgoResult<CsvWriteReport>;
}
```

Strict RFC 4180 conformance: fields with `,`, `"`, CR, or LF are quoted; embedded `"` is doubled (`""`); default `CRLF` line terminator; UTF-8 BOM optional (off by default). The `quote_minimal=false` mode quote-wraps every field — useful for downstream tools that mis-handle minimal quoting.

Type coercion table (when a row's serde_json::Value type doesn't match the column's CsvColumnKind):
- `Text` ← any → format with `to_string()` (`null` becomes `""`)
- `Integer` ← `Number(i)` → `i.to_string()`; ← `String(s)` → parse-or-reject
- `Decimal` ← `Number(f)` → `format!("{:.}", f)`; preserve trailing zeros if string-input
- `Date` ← `String(s)` → parse as RFC 3339 then format as `YYYY-MM-DD`; reject anything else
- `Boolean` ← `Bool(b)` → `"true" | "false"`; ← `String("true"/"false"/"1"/"0")` → coerce
- `EmailAddress` ← `String(s)` → validate `addr@domain.tld` shape; reject otherwise

Sandbox: `path` MUST be inside `[document_csv].output_root` (default `~/.argo/exports/csv/`). Operator can override per-tenant via TOML config. Path-traversal attempts (`../`, absolute paths outside root, symlinks) return typed `ArgoError::SandboxViolation` — NOT silently rewritten.

Worker-action wire-up: NEW `ExecutorKind::DocumentCsv` variant + parse arms (`"document_csv"` / `"document-csv"`) + thin adapter `crates/argo-cli/src/handlers/worker_actions_executor_document_csv.rs` that converts the manifest's action input into a `CsvWriteRequest`, calls `DocumentCsv::write`, and surfaces the report (sha256 + path + row counts + rejections) into the action bundle.

This is NOT in scope: XLSX (separate brief — separate crate `argo-executor-document-xlsx`); PDF (separate); JSON Lines (separate); ODS / Numbers; partial-row repair (rejections are surfaced, not retried); CSV-import / parse (this is write-only); streaming write for >1M-row exports (in-memory only in v1; document the cap).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-executor-document-csv/` (NEW crate, ≤ 700 LOC total)
  - `Cargo.toml` (NEW, ≤ 25 LOC)
  - `src/lib.rs` (NEW, ≤ 60 LOC) — re-exports
  - `src/document_csv.rs` (NEW, ≤ 240 LOC) — `DocumentCsv` + `CsvWriteRequest` + `CsvWriteReport` + RFC 4180 emit
  - `src/types.rs` (NEW, ≤ 120 LOC) — `CsvColumnSchema` + `CsvColumnKind` + `LineTerminator` + `RowRejection`
  - `src/sandbox.rs` (NEW, ≤ 80 LOC) — `[document_csv].output_root` enforcement + path-traversal refusal
  - `src/document_csv_tests.rs` (NEW, sibling, ≤ 280 LOC) — 9 unit tests:
    - rfc-4180-roundtrip (write then re-parse with a stdlib-style CSV reader, expect bytewise-equal-after-normalize)
    - quote-required-on-comma-quote-cr-lf
    - quote-all-mode-quotes-every-field
    - type-coerce-text-integer-decimal-date-boolean-email (one assertion per kind)
    - missing-required-rejects-row-without-aborting-rest
    - max-len-exceeded-rejects-row
    - path-traversal-rejected (`../escape.csv`, absolute path, symlink outside root)
    - empty-rows-writes-header-only
    - sha256-deterministic-across-runs-same-input
- `crates/argo-cli/src/handlers/worker_actions_executor.rs` — narrowly scoped: ADD `DocumentCsv` variant + 2 parse aliases + dispatch arm (≤ 25 LOC delta)
- `crates/argo-cli/src/handlers/worker_actions_executor_document_csv.rs` (NEW, ≤ 200 LOC)
- `crates/argo-cli/src/handlers/worker_actions_executor_document_csv_tests.rs` (NEW, sibling, ≤ 180 LOC) — 5 unit tests on the adapter (manifest parse / sandbox-violation surfaces / typed-rejection surfaces / bundle contains sha256+rows+rejections / dispatch routes correctly)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly scoped: register handler
- `crates/argo-vertical/src/manifest_validate.rs` — narrowly scoped: ADD `"document_csv"` + `"document-csv"` to `KNOWN_EXECUTOR_KINDS` (same pattern as recent #1345 macOS/Linux additions)
- `crates/argo-config/src/sections.rs` — narrowly scoped: ADD `[document_csv]` section w/ `output_root` (default `~/.argo/exports/csv/`)
- `crates/argo-config/src/defaults.rs` — narrowly scoped: ADD default values
- `Cargo.toml` (workspace) — narrowly scoped: add `crates/argo-executor-document-csv` to `members`
- `crates/argo-cli/Cargo.toml` — narrowly scoped: add `argo-executor-document-csv` as path dep

## Files you must NOT touch

- Other executor crates (`argo-executor-computer-use-{macos,linux,windows}`, `argo-executor-filesystem`, etc.)
- `crates/argo-store/migrations/**` — no schema (the bundle surfaces report inline; no DB persist in v1)
- Any vertical content

## Acceptance criteria

- [ ] NEW `crates/argo-executor-document-csv/` crate compiles standalone
- [ ] RFC 4180 roundtrip: write a row with `,`, `"`, CR, LF → parse it back with `csv` crate or stdlib equivalent → identical row out
- [ ] `quote_minimal = false` quote-wraps every field including header
- [ ] All 6 column kinds coerce correctly; type-mismatch rejects the row (not the whole file)
- [ ] `required = true` + missing field → row in `rejections`, NOT in `bytes_written`
- [ ] `max_len = Some(N)` + field longer than N → row rejected
- [ ] Path-traversal (`../`, absolute outside `output_root`, symlink outside) → typed `ArgoError::SandboxViolation`
- [ ] Empty rows + `include_header = true` → file contains just the header line (CRLF-terminated)
- [ ] `sha256` is deterministic — same input → same hash across runs
- [ ] Worker-action adapter dispatches `ExecutorKind::DocumentCsv` and surfaces the report into the action bundle
- [ ] `argo vertical lint <some-vertical>` with `executor.kind = "document_csv"` doesn't reject the kind (manifest_validate.rs allowlist hit)
- [ ] 9 unit tests on document_csv + 5 unit tests on the adapter
- [ ] `cargo clippy -p argo-executor-document-csv -p argo-cli -p argo-vertical -p argo-config --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-executor-document-csv -p argo-cli` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-document-creation-executor-csv`
- PR title: `executor: document-csv (RFC 4180 writer + sandbox + worker dispatch) (mission-document-creation-executor-csv)`

## Notes for the implementer

- Use the `csv` crate (already in workspace lockfile per similar connectors) for both write and the roundtrip-test parse. Don't hand-roll quoting unless `csv` is missing — it's not.
- `LineTerminator::CRLF` is RFC 4180 default and the right choice for max compatibility (Excel chokes on bare LF in some legacy versions). Don't switch to LF as default to "be Unix-correct".
- Sandbox enforcement: canonicalize `path` AFTER tilde-expand AND symlink-resolve, THEN check it has `output_root.canonicalize()?` as a prefix. The `crates/argo-executor-filesystem::sandbox::Sandbox` primitive (existing, mature) already does this — use it, don't reimplement.
- Reject behavior is RECORD-LEVEL, not file-level. One bad row doesn't abort the whole export. The report's `rows_rejected` + `rejections` array gives the operator exactly which rows failed and why.
- `EmailAddress` validation is INTENTIONALLY loose — just `^[^@]+@[^@]+\.[^@]+$` shape check. Don't import `email_address` or `validator` crate for stricter validation; that's a Tier A separate brief if anyone wants it.
- `sha256` should hash the EXACT bytes written, including BOM if BOM enabled. Operators rely on this for downstream verify chains.
- Manifest validation: mirror the just-merged #1345 pattern (Linux executor) — when adding a new ExecutorKind, add the kind name to BOTH `worker_actions_executor.rs::parse_executor_kind` AND `argo-vertical::manifest_validate::KNOWN_EXECUTOR_KINDS`. The brief author for #1345 missed the second list and the adapter caught it; learn from that.
- Document the in-memory cap explicitly in module docs: "writes ≤ 1 M rows in v1; streaming write is a follow-up brief once we see a vertical that actually needs ≥ 1 M rows."
