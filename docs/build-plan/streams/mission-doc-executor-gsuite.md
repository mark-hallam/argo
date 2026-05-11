# Mission load-bearing wire — document-creation executor (Google Workspace: Docs / Sheets / Slides)

**Owner**: claude. **Depends on**: just-shipped `crates/argo-executor-document-csv/` (#1360 MERGED 2026-05-12) as the canonical multi-file executor + worker-action-dispatch pattern; existing `crates/argo-executor-filesystem::Sandbox` primitive for output-path safety.
**Authored against**: origin/main `ea8b4406` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-doc-executor-gsuite`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #102 ("Build document creation executor — Google Docs/Sheets/Slides"). **Tier A (executor).** Today the document-creation executors cover local PDF/markdown/slides (SHIPPED) + CSV (just shipped #1360) + Microsoft Graph (queued via `mission-doc-executor-msgraph`). This brief lands the Google Workspace counterpart — create+edit Google Docs / Sheets / Slides via the Drive + Docs + Sheets + Slides APIs. Drafts-only by design: every action emits a draft document; the operator approves before sharing/publishing.

## Scope

NEW `crates/argo-executor-document-gsuite/` crate (≤ 1100 LOC across 6 files):

- `crates/argo-executor-document-gsuite/Cargo.toml` (NEW, ≤ 35 LOC) — pulls in `reqwest` (rustls-tls), `serde`, `serde_json`, `thiserror`, `tokio`, `chrono`, `argo-types`. Dev-dep `wiremock`.
- `crates/argo-executor-document-gsuite/src/lib.rs` (NEW, ≤ 80 LOC) — re-exports + crate-level rustdoc with usage example.
- `crates/argo-executor-document-gsuite/src/document_gsuite.rs` (NEW, ≤ 340 LOC) — `DocumentGsuite::write(GsuiteWriteRequest) -> ArgoResult<GsuiteWriteReport>`. Three document kinds:
  - `Doc` — create-then-batch-update with `insertText` / `updateTextStyle` / `createParagraphBullets` requests.
  - `Sheet` — create + `batchUpdate` with `appendCells` for typed rows (re-uses the CSV crate's `CsvColumnKind` enum for cell types).
  - `Slides` — create + `batchUpdate` with `createSlide` / `insertText` / `createImage` requests (image-by-URL only; uploaded blobs are a v2).
- `crates/argo-executor-document-gsuite/src/auth.rs` (NEW, ≤ 180 LOC) — OAuth2 service-account JWT flow (single-shot token mint + 5-min-before-expiry refresh + on-401 force-refresh). Mirrors the Microsoft Graph teams.rs token pattern (#1362). `client_email` + `private_key_pem` from env-indirect (`ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL` + `ARGO_GSUITE_SERVICE_ACCOUNT_KEY`).
- `crates/argo-executor-document-gsuite/src/types.rs` (NEW, ≤ 130 LOC) — `GsuiteDocKind { Doc, Sheet, Slides }`, `GsuiteWriteRequest`, `GsuiteWriteReport`, `GsuiteDocId`, `GsuiteSandboxError`. The report includes `doc_id`, `web_view_link`, `mime_type`, `created_at`, `sha256_of_request` (deterministic for the same logical request).
- `crates/argo-executor-document-gsuite/src/document_gsuite_tests.rs` (NEW, ≤ 280 LOC) — 10 unit tests using `wiremock`: happy-path Doc create, happy-path Sheet create w/ 3 typed rows, happy-path Slides create w/ 2 slides, auth-401-force-refresh, token-cache-hit-no-refresh, malformed-row-typed-rejection, sandbox-doc-id-empty-rejection, image-URL-validation-rejection, slide-image-with-invalid-https-rejected, batch-update partial-failure surfaces `rejections` array.

NEW worker-action dispatch + manifest-validator + config wire-up:

- EXTEND `crates/argo-cli/src/handlers/mod.rs` (≤ 3 LOC).
- EXTEND `crates/argo-cli/src/handlers/worker_actions_executor.rs` (≤ 15 LOC delta) — register `ExecutorKind::DocumentGsuite` variant alongside CSV.
- NEW `crates/argo-cli/src/handlers/worker_actions_executor_document_gsuite.rs` (NEW, ≤ 175 LOC) — handler that parses the worker-action payload + dispatches into `DocumentGsuite::write`.
- NEW `crates/argo-cli/src/handlers/worker_actions_executor_document_gsuite_tests.rs` (NEW, ≤ 140 LOC) — 5 handler tests.

NEW `crates/argo-config/` extensions:
- EXTEND `crates/argo-config/src/sections.rs` (≤ 35 LOC) — `[document_gsuite]` section: `default_drive_folder_id` (Drive folder operators want all drafts inside; if blank, the service-account's "My Drive" is used), `permitted_doc_kinds = ["Doc","Sheet","Slides"]`, `max_doc_size_kb = 256`.
- EXTEND `crates/argo-config/src/defaults.rs` (≤ 8 LOC) — `DEFAULT_GSUITE_DOC_KINDS`, etc.
- EXTEND `crates/argo-config/src/lib.rs` (≤ 5 LOC) — re-export.

NEW manifest-validator entry:
- EXTEND `crates/argo-vertical/src/manifest_validate.rs` (≤ 4 LOC) — register `document_gsuite` as a known executor kind for vertical action declarations.

NEW integration smoke `crates/argo-cli/tests/document_gsuite_smoke.rs` (NEW, ≤ 220 LOC) — env-gated (`ARGO_TEST_GSUITE_*`): creates a doc + sheet + slides; verifies `web_view_link` is reachable; verifies the sandbox-folder placement; cleans up via Drive trash API.

NO migration. NO rewrite of the CSV executor or doc-creation-executor-document (existing local PDF/markdown crate). NO change to the auth flow of any other executor.

This is NOT in scope: Google Drive folder management (the executor accepts a `default_drive_folder_id`; folder creation is out-of-scope — operator creates the folder once in Drive); image-blob upload (only image-by-URL in v1); Forms / Sites / Apps Script (out-of-scope; separate brief if needed); user-delegation (the service-account is the actor; Workspace-domain-wide-delegation is operator-side setup); per-doc revision-history queries; PDF export (the executor creates Google-native docs; PDF export is a Drive-API follow-up).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-executor-document-gsuite/Cargo.toml` (NEW, ≤ 35 LOC)
- `crates/argo-executor-document-gsuite/src/lib.rs` (NEW, ≤ 80 LOC)
- `crates/argo-executor-document-gsuite/src/document_gsuite.rs` (NEW, ≤ 340 LOC)
- `crates/argo-executor-document-gsuite/src/auth.rs` (NEW, ≤ 180 LOC)
- `crates/argo-executor-document-gsuite/src/types.rs` (NEW, ≤ 130 LOC)
- `crates/argo-executor-document-gsuite/src/document_gsuite_tests.rs` (NEW, ≤ 280 LOC, included via `#[path]`)
- `crates/argo-cli/src/handlers/mod.rs` (EXTEND ≤ 3 LOC)
- `crates/argo-cli/src/handlers/worker_actions_executor.rs` (EXTEND ≤ 15 LOC)
- `crates/argo-cli/src/handlers/worker_actions_executor_document_gsuite.rs` (NEW, ≤ 175 LOC)
- `crates/argo-cli/src/handlers/worker_actions_executor_document_gsuite_tests.rs` (NEW, ≤ 140 LOC)
- `crates/argo-cli/Cargo.toml` (EXTEND ≤ 4 LOC) — depend on new crate
- `crates/argo-cli/tests/document_gsuite_smoke.rs` (NEW, ≤ 220 LOC)
- `crates/argo-config/src/sections.rs` (EXTEND ≤ 35 LOC)
- `crates/argo-config/src/defaults.rs` (EXTEND ≤ 8 LOC)
- `crates/argo-config/src/lib.rs` (EXTEND ≤ 5 LOC)
- `crates/argo-vertical/src/manifest_validate.rs` (EXTEND ≤ 4 LOC)
- `Cargo.toml` (workspace EXTEND — add `argo-executor-document-gsuite` to `members`)
- `Cargo.lock` (auto-regenerated; do not hand-edit)

## Files you must NOT touch

- `crates/argo-executor-document-csv/**` (existing CSV crate stays)
- `crates/argo-executor-document/**` (existing local doc executor stays)
- `crates/argo-store/migrations/**` (no migration)
- Any vertical manifest

## Acceptance criteria

- [ ] `cargo build -p argo-executor-document-gsuite` succeeds
- [ ] `cargo test -p argo-executor-document-gsuite` passes all 10 unit tests
- [ ] `cargo test -p argo-cli worker_actions_executor_document_gsuite` passes all 5 handler tests
- [ ] `KNOWN_EXECUTOR_KINDS` (or equivalent) includes `document_gsuite` after this PR
- [ ] `argo vertical lint <any-vertical>` accepts an action declaring `executor = "document_gsuite"`
- [ ] Service-account auth refreshes 5 min before token expiry; on 401 force-refresh
- [ ] Secrets redacted from any error-message logging (private_key_pem never in stderr)
- [ ] Per-doc-kind type validation (Sheet rows use the CSV crate's typed-cell coercion; Doc text inserts respect API request limits; Slides image-URLs validated HTTPS-only)
- [ ] All new files < 500 LOC; per-file ≤ briefed budgets
- [ ] No migration; no changes to existing executors
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-doc-executor-gsuite`
- PR title: `executor: document-gsuite (Drive + Docs + Sheets + Slides via service-account OAuth2) (mission-doc-executor-gsuite)`

## Notes for the implementer

- Treat just-merged `crates/argo-executor-document-csv/` (#1360) as the canonical pattern for crate layout + worker-action dispatch + config-section wiring. The new crate's structure should mirror it 1:1; the only delta is the API layer (Google Workspace HTTP vs local FS).
- Treat `crates/argo-channels/src/teams.rs` (#1362) as the canonical pattern for service-account OAuth2 + token-cache + refresh-on-401. Reuse the structure; don't reinvent.
- The `Sheet` cell-type coercion re-uses `argo-executor-document-csv::CsvColumnKind` — depend on the CSV crate as a path-dep. This is the load-bearing reuse: typed cells should behave identically in CSV and Sheet outputs.
- Image-URL validation for Slides: must be HTTPS, must respond `200` to a HEAD request, must be `image/*` mime. Reject otherwise. Document this in the executor's rustdoc.
- `default_drive_folder_id` — if blank, the doc lands in service-account's My Drive. Operators wanting tenant-isolation should create per-tenant folders + share with the service-account; the brief assumes this is set up out-of-band.
- `permitted_doc_kinds` config gates which kinds the executor will produce. Default `["Doc","Sheet","Slides"]`. Operator can restrict to `["Doc"]` for verticals that should only emit text drafts.
- The `sha256_of_request` field in `GsuiteWriteReport` provides deterministic round-trip identity: same logical request → same hash. Useful for the audit-chain integration (per the existing per-executor audit pattern).
- Token caching is per-process — when `argo gateway` boots, the first `document_gsuite` action mints; subsequent ones reuse. The service-account credential never leaves the process; only the bearer token (in memory).
- DO NOT add support for user-delegation (`subject = "user@..."` in JWT). That's Workspace-domain-wide-delegation; operator-side setup; v2 if needed.
- DO NOT add a "create-folder" subcommand. Drive folder management is out-of-scope; operator creates folders in the Drive UI.
- DO NOT add PDF export. The executor creates Google-native docs; PDF export via Drive API is a separate brief if operators ask.
- The `wiremock` test setup should mock the OAuth token endpoint + the Drive/Docs/Sheets/Slides API endpoints. Document the mock structure in the crate's README example so operators authoring follow-on tests know the recipe.
- The integration smoke test is env-gated (`ARGO_TEST_GSUITE_SERVICE_ACCOUNT_EMAIL` + `ARGO_TEST_GSUITE_SERVICE_ACCOUNT_KEY` present → run; absent → silent skip per the canonical pattern). Document this in the runbook context.
