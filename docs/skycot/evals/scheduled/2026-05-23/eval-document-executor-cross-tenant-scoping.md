# Eval — `argo-executor-document(-csv,-gsuite)` cross-tenant scoping

**Brief**: `eval-document-executor-cross-tenant-scoping`
**Authored against**: `origin/main` at the worktree HEAD of the claim commit (`36cd99555`).
**Scope**: read-only audit of `crates/argo-executor-document/src/**`, `crates/argo-executor-document-csv/src/**`, `crates/argo-executor-document-gsuite/src/**`, and the two `argo-cli` dispatch shims that wire them (`worker_actions_executor_document.rs`, `worker_actions_executor_document_gsuite.rs`).
**Severity scale**: P0 — ship blocker / data-integrity / cross-tenant leak; P1 — substantive correctness gap; P2 — quality / completeness; P3 — minor.
**Companion brief**: gap manifest row at `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` (Claim 4 — Skills shared + learned).

Cite per the brief notes: the gap-manifest row reads —

> | `argo-executor-document(-csv,-gsuite)` | ? | 🔴 NOT AUDITED | Document executors (CSV reader, Google Sheets). | — | 🟡 P2 | `eval-document-executor-cross-tenant-scoping` — Sheets credentials per tenant; CSV file reads scoped to worker's `worker_fs_root`. |

---

## TL;DR

The three crates implement document-creation primitives (markdown / CSV / PDF / slides / OneDrive / Google Workspace). Every surface is **tenant-blind by construction** — `grep -rn 'tenant_id\|TenantId' crates/argo-executor-document*/` returns **zero hits**. That is acceptable in SKYCOT's documented single-tenant-per-process model, where the gateway's cwd is the tenant boundary by convention. It becomes a **🟡 P2 cross-tenant leak surface** the moment a gateway process serves multiple tenants from a single cwd — which is the architecture the per-tenant SQL tables in `argo-store` (`worker_fs_root`, `workspace_provider_key`) are already shaped for.

Six findings, all FINDINGS-ONLY (no fix in this brief). No P0 inside the three crates: the structural risk lives at the seam between these executors and the gateway. P0 escalation depends on whether the gateway process is ever multi-tenant; today it is not, but the rest of SKYCOT is being engineered toward that.

| # | Finding | Severity | Cite |
|---|---|---|---|
| F-1 | No `TenantId` on any public surface — `DocExecutor::run`, `MsGraphExecutor::create`, `DocumentCsv::write`, `DocumentGsuite::write`, `OutputPath::resolve`. | 🟡 P2 (P0 if gateway goes multi-tenant per cwd) | `executor.rs:191`, `msgraph.rs:131`, `document_csv.rs:95`, `document_gsuite.rs:94` |
| F-2 | PDF intermediate HTML lands at `<bundle>/.staging/render.html` — sibling to the output, no tenant tag. Bundle-id collision = co-tenant readable plaintext. | 🟡 P2 | `pdf.rs:78-88` |
| F-3 | `MsGraphClient::from_env()` reads a single process-wide `ARGO_MSGRAPH_ACCESS_TOKEN`. Every tenant on the process uploads to the same OneDrive identity. | 🟡 P2 | `msgraph.rs:82-90` |
| F-4 | `GsuiteAuthConfig::from_env()` reads a single process-wide `ARGO_GSUITE_SERVICE_ACCOUNT_*`. Every tenant on the process writes to the same Drive identity. | 🟡 P2 | `auth.rs:66-89`; dispatch shim docstring `worker_actions_executor_document_gsuite.rs:11-14` already concedes this. |
| F-5 | CSV `DEFAULT_OUTPUT_ROOT = "~/.argo/exports/csv/"`. No per-tenant subdir; the sandbox guards `..` traversal only, not in-root tenant separation. Tenant A can pick a path that lands inside tenant B's namespace. | 🟡 P2 | `sandbox.rs:27`; `document_csv.rs:95-97` |
| F-6 | Dispatch shims build the bundle as `cwd.join(".skycot/outputs/<job_id>/actions/<action_id>/")` with no `tenant_id` segment. If two tenants' manifests can coin colliding `job_id` (forged or accidental), bundle writes interleave. | 🟡 P2 | `worker_actions_executor_document.rs:103-117`; `worker_actions_executor_document_gsuite.rs:95-109` |

Companion fix-briefs are *not* spun out in this PR — the executors work as designed for the v1 single-tenant-per-cwd model and the brief authored only the audit. When the gateway grows multi-tenant cwd reuse, each row above maps to a follow-up brief (see §7).

---

## §1 — Surface inventory

### `argo-executor-document` (1 514 LOC across 13 source files)

| File | LOC | Role |
|---|---:|---|
| `src/lib.rs` | 49 | Public re-exports |
| `src/executor.rs` | 381 | `DocExecutor`, `DocRequest`, `OutputPath::resolve` |
| `src/markdown.rs` | 46 | Markdown writer primitive |
| `src/csv.rs` | 110 | Thin RFC 4180 shim — delegates to `argo-executor-document-csv::render_rfc4180` |
| `src/pdf.rs` | 183 | Markdown → HTML → headless-Chrome PDF |
| `src/slides.rs` | 168 | reveal.js HTML slide deck |
| `src/msgraph.rs` | 397 | Microsoft Graph OneDrive client + Open-XML builders |
| `src/msgraph_tests.rs` | 116 | wiremock tests |
| `src/error.rs` | 62 | `DocExecutorError` |
| `src/extract/mod.rs` | 159 | Document-extraction request/response shape |
| `src/extract/claude_vision.rs` | 159 | Anthropic Claude vision provider |
| `src/extract/provider.rs` | 85 | `ExtractionProvider` trait |
| `src/extract/schema.rs` | 194 | Operator JSON-Schema validator |
| `src/extract/tables.rs` | 274 | Table parsing |

### `argo-executor-document-csv` (854 LOC across 6 files)

`DocumentCsv::write` (`document_csv.rs:95`) + sandbox path resolver (`sandbox.rs:66`) + 6-kind coercion table (`document_csv.rs:223`). Sandbox roots default to `~/.argo/exports/csv/` (`sandbox.rs:27`).

### `argo-executor-document-gsuite` (1 532 LOC across 6 files)

`DocumentGsuite::write` (`document_gsuite.rs:94`) + RS256 JWT bearer-token cache (`auth.rs:149`). Drive `files.create` + per-kind `batchUpdate` against Docs / Sheets / Slides. Image-URL preflight (`document_gsuite.rs:268`).

### What is NOT present in any of the three crates

- **No `TenantId` parameter on any public function.** `grep -rn 'TenantId\|tenant_id' crates/argo-executor-document*/` returns zero hits.
- **No tenant-keyed credential lookup.** Both `MsGraphClient::from_env()` and `GsuiteAuthConfig::from_env()` are 0-arg constructors reading process-wide env vars.
- **No multi-tenant cache keying.** The Gsuite `GsuiteAuth` `RwLock<Option<CachedToken>>` (`auth.rs:133`) is a single cell per process.
- **No `worker_fs_root` integration.** The brief's gap-manifest text suggests CSV file reads should be scoped to `worker_fs_root` (per migration 79); they are not. The CSV sandbox uses an operator-configured `output_root`, not the per-worker FS root.
- **No tenant component in the bundle path.** Bundle dir is `cwd/.skycot/outputs/<job_id>/actions/<action_id>/` (both dispatch shims).

---

## §2 — Per-finding detail

### F-1 — `TenantId`-blind public surface

Five entry points should plausibly carry a tenant id once the gateway is multi-tenant:

- `DocExecutor::run(&self, request: &DocRequest, bundle: &Path) -> DocResult<DocOutcome>` (`executor.rs:191`)
- `MsGraphExecutor::create(&self, fmt: MsGraphFormat, path: &str, bytes: &[u8]) -> Result<MsGraphFile, MsGraphError>` (`msgraph.rs:131`)
- `MsGraphExecutor::read(&self, path: &str) -> Result<Vec<u8>, MsGraphError>` (`msgraph.rs:167`)
- `DocumentCsv::write(&self, req: CsvWriteRequest) -> ArgoResult<CsvWriteReport>` (`document_csv.rs:95`)
- `DocumentGsuite::write(&self, req: GsuiteWriteRequest) -> ArgoResult<GsuiteWriteReport>` (`document_gsuite.rs:94`)

All five take ownership of input + path strings but no tenant context. The crates rely on the caller (the `argo-cli` dispatch shim) to compute a tenant-safe `bundle`/`path`/`output_root`. Today no caller does.

**Codified as**: `crates/argo-executor-document/tests/eval_findings_cross_tenant.rs::doc_executor_run_signature_takes_no_tenant_context`, plus parallel structural assertions in the sibling crates' findings tests.

### F-2 — PDF `.staging/render.html` co-tenant readable

`render_pdf` (`pdf.rs:78`) writes the source markdown rendered to HTML into `<bundle>/.staging/render.html` so the operator can inspect it post-mortem. With shared `cwd` and predictable `job_id`/`action_id` segments, a second tenant can `cat` the staging file and recover the full markdown body. The output PDF is content-equivalent to the HTML, so the leak is logically additive (the PDF would already be readable), but `.staging/render.html` is also written even when the caller intends to delete the bundle on success — and there is no delete in `render_pdf`.

**Codified as**: `pdf_render_writes_staging_html_with_full_body_no_tenant_segment`.

### F-3 — `MsGraphClient::from_env()` is process-global

```rust
// crates/argo-executor-document/src/msgraph.rs:82
pub fn from_env() -> Result<Self, MsGraphError> {
    match std::env::var("ARGO_MSGRAPH_ACCESS_TOKEN") { ... }
}
```

There is no `from_env_for_tenant(tenant: TenantId)` overload. Every tenant on the process shares one OneDrive identity. Gsuite's same pattern is **F-4**.

The right shape (when the gateway goes multi-tenant) is to plumb credentials through `argo-credentials::MasterKey::resolve_for_tenant` (already landed per the recent `mission-credentials-per-tenant-master-key` brief noted in `crates/argo-credentials/tests/eval_findings_cross_tenant.rs:21-29`) and read the bearer token from the per-tenant credential store, not from process env. Today neither MsGraph nor Gsuite touches `argo-credentials`.

**Codified as**: `msgraph_client_from_env_returns_one_process_wide_token`.

### F-4 — `GsuiteAuthConfig::from_env()` is process-global

```rust
// crates/argo-executor-document-gsuite/src/auth.rs:66
pub fn from_env() -> ArgoResult<Self> {
    let client_email = std::env::var("ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL")...;
    let private_key_pem = std::env::var("ARGO_GSUITE_SERVICE_ACCOUNT_KEY")...;
    ...
}
```

The dispatch shim's docstring already concedes this:

> Auth comes from process-level env (the `ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL` and `ARGO_GSUITE_SERVICE_ACCOUNT_KEY` vars); the executor never inlines credentials into the payload. Operators rotate by re-issuing the service-account key and bouncing the gateway.

(`worker_actions_executor_document_gsuite.rs:11-14`)

Bouncing the gateway to rotate is a single-tenant pattern. Multi-tenant deployments need per-tenant service accounts, refreshed without restart.

**Codified as**: `gsuite_auth_config_from_env_is_process_global`.

### F-5 — CSV `DEFAULT_OUTPUT_ROOT` has no tenant subdir

```rust
// crates/argo-executor-document-csv/src/sandbox.rs:27
pub const DEFAULT_OUTPUT_ROOT: &str = "~/.argo/exports/csv/";
```

The sandbox primitive (`resolve_sandboxed_path`) guards `..` traversal and absolute paths whose canonical prefix is outside the root. It offers **no in-root tenant separation**. A worker controlled by tenant A whose CSV-emit action has `path = "tenant-b-leak.csv"` writes to `~/.argo/exports/csv/tenant-b-leak.csv` — the same path tenant B would read from.

The right shape is `DEFAULT_OUTPUT_ROOT.join(tenant_id).join(path)` and a `Sandbox::open_for_tenant(root, tenant_id)` primitive. Today the operator must set a per-tenant `output_root` in config; the gap manifest's "CSV file reads scoped to worker's `worker_fs_root`" hint is the documented v1.x direction.

**Codified as**: `csv_default_output_root_has_no_tenant_segment`.

### F-6 — Bundle path is cwd-relative, not tenant-relative

```rust
// crates/argo-cli/src/handlers/worker_actions_executor_document.rs:103
fn ensure_bundle_dir(cwd: &Path, job_id: &str, action_id: &str) -> ArgoResult<PathBuf> {
    let dir = cwd
        .join(".skycot")
        .join("outputs")
        .join(job_id)
        .join("actions")
        .join(action_id);
    ...
}
```

`job_id` and `action_id` come from the action manifest. If the gateway grows multi-tenant cwd reuse and `job_id` is operator-supplied (not minted per tenant), two tenants can name the same bundle and step on each other's outputs / staging html / msgraph upload buffer. Today `job_id` is a `WorkerSessionId`-derived UUID, so practical collision risk is low — the structural risk is that nothing in the path *guarantees* tenant separation.

**Note**: this finding lives in the `argo-cli` dispatch shim, not in `argo-executor-document/`. It is included here because the executors trust their caller's bundle path implicitly — moving tenant logic into the executor (per F-1) would let the executor enforce the segment regardless of who calls.

---

## §3 — Closing the loop

When (not if) the gateway goes multi-tenant per cwd, the follow-up briefs are:

1. `mission-document-executors-per-tenant-credentials` — fold `MsGraphClient::from_env` and `GsuiteAuthConfig::from_env` into per-tenant credential lookups via `argo-credentials::MasterKey::resolve_for_tenant`. P1 once gateway turns the flag.
2. `mission-document-executors-bundle-path-tenant-segment` — extend `ensure_bundle_dir` in both dispatch shims to interpolate `tenant_id` before `job_id`. P1.
3. `mission-document-csv-output-root-per-tenant` — pin `DEFAULT_OUTPUT_ROOT` to a tenant-scoped subdir and add `Sandbox::open_for_tenant`. P1.
4. `mission-document-pdf-staging-cleanup-and-tenant-tag` — delete `.staging/render.html` on success and prefix its name with a tenant tag for partial failures. P2.
5. `mission-document-executor-trait-takes-tenant-id` — when the wider `Executor` trait adds a `tenant_id: TenantId` param (already on `mission-cross-tenant-worker-handler-no-existence-leak`'s trajectory), propagate that change here. P2 — coordinate.

The FINDINGS tests in this PR carry these brief names verbatim in their `#[ignore = "..."]` docstrings so a fix lands a single-line `s/#\[ignore = "FINDINGS-ONLY: .*"\]/\/\/ FINDINGS finding closed by <brief>/` removal.

---

## §4 — What I did NOT touch

- **No production source edits.** The crates' code is structurally correct for the v1 single-tenant-per-cwd model; widening the public surface to take `TenantId` before the gateway needs it would be premature.
- **No `argo-cli` dispatch-shim edits.** F-6 lives there; the shim is currently outside the brief's may-touch list and the fix is one of the follow-up briefs (§3).
- **No `argo-redact` / DLP work.** Out of scope; covered by `eval-redact-crate-promote-from-regex-to-real-fence`.
- **No `argo-store` migration.** Per-tenant `worker_fs_root` already exists (migration 79); the gap is the executors ignoring it, not a missing table.
