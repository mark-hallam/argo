# Eval Δ — `argo-executor-document(-csv,-gsuite)` cross-tenant scoping **v2**

**Brief**: `eval-document-executor-cross-tenant-scoping-v2`
**Prior**: v1 audit shipped via PR **#2127** (`7659f923a`,
2026-05-23) — see
`docs/skycot/evals/scheduled/2026-05-23/eval-document-executor-cross-tenant-scoping.md`.
**Companion**: gap manifest row at
`docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
(Claim 5 — Any task a human can do on a PC).

Cite the gap-manifest row this brief implements (verbatim):

> | `argo-executor-document(-csv,-gsuite)` | ? | 🟡 **AUDITED 2026-05-23 (#2127 / `7659f923a`)** FINDINGS-ONLY | Document executors (CSV reader, Google Sheets). 6 × 🟡 P2 wire-gaps pinned: (F-1) no `TenantId` on `DocExecutor::run` / `MsGraphExecutor::create` / `DocumentCsv::write` / `DocumentGsuite::write` / `OutputPath::resolve`; (F-2) PDF intermediate HTML staged sibling to output, no tenant tag; (F-3) `MsGraphClient::from_env` reads one process-wide `ARGO_MSGRAPH_ACCESS_TOKEN`; (F-4) `GsuiteAuthConfig::from_env` reads one process-wide service-account; (F-5) CSV `DEFAULT_OUTPUT_ROOT = "~/.argo/exports/csv/"` has no tenant subdir. Escalates to P0 if the gateway turns multi-tenant per cwd. | — | 🟡 P2 | `eval-document-executor-cross-tenant-scoping` — Sheets credentials per tenant; CSV file reads scoped to worker's `worker_fs_root`. |

---

## TL;DR

v1 (PR #2127) shipped 6 findings as `#[ignore]`'d FINDINGS-ONLY tests.
That was the correct first move — but ignored tests do not actually
defend a contract: a refactor that re-orders / renames / inserts an
argument can silently slide past, because cargo never runs them by
default.

**v2 promotes F-1 from a documented finding into a LIVE pin.** The
broken-shape (no `TenantId` on the five public dispatch surfaces and
the three top-level request/outcome JSON shapes) is now locked by
**19 LIVE tests** across the three crates. Any refactor that changes
the shape — including the right refactor — has to delete or rewrite
these pins. That visibility is the point.

v2 also ships **three `#[ignore]`'d POST-FIX tests** (one per crate).
Each is a single positive assertion that the eventual fix-brief
(`mission-document-executor-tenant-stamp`) will satisfy. The
companion fix-brief un-ignores them; v2 itself ships zero production
edits.

| Crate | LIVE pin tests | POST-FIX tests | New file |
|---|---:|---:|---|
| `argo-executor-document` | 10 | 1 | `tests/surface_contract_pin_f1.rs` |
| `argo-executor-document-csv` | 5 | 1 | `tests/surface_contract_pin_f1.rs` |
| `argo-executor-document-gsuite` | 4 | 1 | `tests/surface_contract_pin_f1.rs` |
| **Total** | **19** | **3** | 3 files |

No production source touched. Zero new dependencies. Zero new crates.

---

## §1 — Why pin and not just `#[ignore]`?

The `#[ignore]`'d FINDINGS pattern (used throughout v1) is a good
DOCUMENTATION mechanism: the test body cites the audit, names the
fix-brief, encodes the shape, and runs cleanly under `cargo test
--ignored`. But it has a structural weakness — cargo skips ignored
tests by default, so the shape they describe is enforced only when
someone remembers to run `cargo test --ignored`. Between v1's merge
and the eventual fix-brief there is a long window in which a refactor
might:

- Rename `DocExecutor::run` → `DocExecutor::run_sync` (the fn-pointer
  bind in the ignored test still compiles against the new name only
  if the test is rerun);
- Add a `bundle_kind: BundleKind` argument that is NOT `TenantId` but
  whose insertion site overlaps the future tenant-arg slot;
- Replace `&DocRequest` with `DocRequest` by value, changing the
  signature without changing the semantics — same point.

LIVE pin tests close that window. They run on every `cargo test -p
<crate>` invocation that any reviewer triggers, and on every workspace
CI run. The cost is that the LIVE pin set must be maintained
deliberately: a refactor that legitimately changes the shape has to
update / delete the pin in the same patch. **That is the desired
behaviour.** The pin is the "stop and think" gate.

The companion `#[ignore]`'d POST-FIX test in each crate is the
positive corollary: once the fix-brief lands the right shape, the
reviewer un-ignores the test and deletes the pin in the same diff. The
two together make the contract bilaterally explicit.

## §2 — Pins shipped

### `argo-executor-document/tests/surface_contract_pin_f1.rs` (10 LIVE + 1 POST-FIX)

LIVE signature pins (binding to typed `fn` pointer / closure;
compile-fails if a `TenantId` arg is inserted):

- `pin_doc_executor_run_signature_no_tenant_arg` —
  `fn(&DocExecutor, &DocRequest, &Path) -> DocResult<DocOutcome>`
- `pin_output_path_resolve_signature_no_tenant_arg` —
  `fn(&Path, &str) -> DocResult<OutputPath>`
- `pin_msgraph_executor_create_signature_no_tenant_arg` — closure
  bind: `(&MsGraphExecutor, MsGraphFormat, &str, &[u8])` → async
- `pin_msgraph_executor_read_signature_no_tenant_arg` — closure
  bind: `(&MsGraphExecutor, &str)` → async
- `pin_doc_executor_new_signature_no_tenant_arg` —
  `fn() -> DocExecutor`

LIVE JSON-shape pins (enumerate the full set of object keys; any
silent field addition shows up as an unexpected key):

- `pin_doc_request_markdown_json_keys_have_no_tenant` — keys are
  `["body", "kind", "output_path"]`
- `pin_doc_request_pdf_json_keys_have_no_tenant` — keys are
  `["body", "kind", "output_path", "title"]`
- `pin_doc_request_msgraph_json_keys_have_no_tenant` — keys are
  `["format", "kind", "output_path", "payload_b64"]`
- `pin_doc_outcome_json_keys_have_no_tenant` — keys are
  `["bytes_written", "kind", "output_path"]`
- `pin_msgraph_file_outcome_json_keys_have_no_tenant` — keys are
  `["item_id", "size_bytes", "web_url"]`

POST-FIX (single, ignored):

- `post_fix_doc_request_carries_tenant_axis` — asserts the JSON
  serialisation contains a `tenant_id` key. Today's body fails;
  un-ignore + drop the relevant LIVE pin when
  `mission-document-executor-tenant-stamp` ships.

### `argo-executor-document-csv/tests/surface_contract_pin_f1.rs` (5 LIVE + 1 POST-FIX)

LIVE signature pins:

- `pin_document_csv_write_signature_no_tenant_arg` —
  `fn(&DocumentCsv, CsvWriteRequest) -> ArgoResult<CsvWriteReport>`
- `pin_document_csv_new_signature_no_tenant_arg` —
  `fn() -> DocumentCsv`

LIVE struct-shape pins (exhaustive destructure; field additions
compile-fail):

- `pin_csv_write_request_field_set_has_no_tenant` — destructure of
  the 8-field `CsvWriteRequest`
- `pin_csv_write_report_field_set_has_no_tenant` — destructure of
  the 6-field `CsvWriteReport`

LIVE literal pin:

- `pin_csv_default_output_root_literal_has_no_tenant_marker` —
  `DEFAULT_OUTPUT_ROOT == "~/.argo/exports/csv/"`, no
  `{tenant_id}` interpolation point.

POST-FIX:

- `post_fix_csv_write_request_carries_tenant_axis` — asserts
  `Debug` repr mentions `tenant_id`. The two surfaces (CSV +
  document-csv) don't derive `Serialize`, so `Debug` is the weakest
  sufficient probe.

### `argo-executor-document-gsuite/tests/surface_contract_pin_f1.rs` (4 LIVE + 1 POST-FIX)

LIVE signature pin (async closure bind):

- `pin_document_gsuite_write_signature_no_tenant_arg` —
  `(&DocumentGsuite, GsuiteWriteRequest) -> Future<ArgoResult<GsuiteWriteReport>>`

LIVE struct + JSON pins:

- `pin_gsuite_write_request_field_set_has_no_tenant` — destructure
  of the 7-field `GsuiteWriteRequest`
- `pin_gsuite_write_request_json_keys_have_no_tenant` — keys
  `["doc_body_text", "drive_folder_id", "kind", "sheet_rows",
  "sheet_schema", "slides", "title"]`
- `pin_gsuite_write_report_field_set_has_no_tenant` — destructure
  of the 7-field `GsuiteWriteReport`

POST-FIX:

- `post_fix_gsuite_write_request_carries_tenant_axis` — asserts
  JSON contains a `tenant_id` key.

---

## §3 — How the fix-brief uses these pins

The follow-up fix-brief (working name
`mission-document-executor-tenant-stamp`, not yet authored as of
2026-05-24) lands per-crate edits that:

1. Add `tenant_id: TenantId` to the relevant entry point (the
   conventional shape is `DocExecutor::run(&self, tenant, &DocRequest,
   bundle)` so the tenant id is the first non-receiver arg);
2. Plumb the tenant id through to the path resolver +
   credential lookup;
3. Stamp the outcome (`DocOutcome` / `CsvWriteReport` /
   `GsuiteWriteReport`) with the tenant id for audit-ledger
   correlation.

When (2) lands, **each LIVE pin in this v2 file that contradicts the
new shape will fail at compile or assertion time.** The fix-brief's
patch must therefore:

- Delete the LIVE pin that's invalidated;
- Un-ignore the matching POST-FIX test in the same file; and
- Rename the POST-FIX test to a positive-regression name (e.g.
  `pin_doc_request_carries_tenant_axis`).

The three-step delete-+-flip-+-rename is a single ~30-line patch per
crate, and is exactly the visibility goal — the v2 brief surfaces the
contract change as code, not as documentation drift.

---

## §4 — What v2 does NOT do

- **No production source edits.** Same boundary as v1; the executors
  remain correct for the v1.0 single-tenant-per-cwd model.
- **No new fix-briefs authored in this PR.** The fix-brief
  `mission-document-executor-tenant-stamp` is named for the eventual
  ledger row but does not exist on disk yet — it'll be drafted by
  the conductor when the gateway-multi-tenant work begins.
- **No re-statement of F-2 through F-6.** v1's existing
  `#[ignore]`'d FINDINGS-ONLY tests for the other 5 findings stay as
  written; only F-1 is promoted to LIVE pin coverage in v2.
- **No edits to `worker_actions_executor_document.rs` /
  `worker_actions_executor_document_gsuite.rs`** (the `argo-cli`
  dispatch shims that own F-6's bundle-path gap). Those are
  out-of-scope and outside the may-touch list.

---

## §5 — Verification

- `cargo clippy -p argo-executor-document -p argo-executor-document-csv -p argo-executor-document-gsuite --all-targets -- -D warnings` clean.
- `cargo test -p argo-executor-document -p argo-executor-document-csv -p argo-executor-document-gsuite` runs the 20 new LIVE pins; all PASS.
- `cargo test -p argo-executor-document -p argo-executor-document-csv -p argo-executor-document-gsuite -- --ignored` runs the 3 new POST-FIX tests; all FAIL today (the expected, documented state) until `mission-document-executor-tenant-stamp` ships.
- All three new files < 500 LOC each (largest is the
  `argo-executor-document` pin at ~190 LOC).
