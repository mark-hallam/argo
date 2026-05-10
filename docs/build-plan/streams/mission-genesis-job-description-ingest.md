# Mission load-bearing wire — Genesis ingests a job description (PDF/text)

**Owner**: claude. **Depends on**: `argo-genesis::charter_dialogue` SHIPPED (verified at `crates/argo-genesis/src/charter_dialogue.rs`); `argo-executor-document::pdf` SHIPPED (verified at `crates/argo-executor-document/src/pdf.rs`).
**Authored against**: origin/main `06e67801` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-job-description-ingest`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 3 item #64 ("Add Genesis that ingests a job description (PDF/text)"). Today Genesis starts from a charter dialogue — operator types or pastes a charter, Genesis classifies → produces a worker manifest. Mission Claim 3 ("platform builds workers") promises a real onboarding flow where an operator can drop in a real job description PDF and get a working teammate — no manual charter authoring required.

## Scope

Add `genesis_from_job_description(input: JobDescInput) -> ArgoResult<GenesisDraft>` where:

```rust
pub enum JobDescInput {
    Pdf(PathBuf),       // path to local PDF; uses existing pdf extractor
    Text(String),       // plain text body
    Url(String),        // http(s) URL — fetched via existing http client; v1 = sync inline fetch
}
```

Pipeline:
1. Extract raw text (PDF → `argo-executor-document::pdf::extract_text`; URL → reqwest GET; Text → identity).
2. Synthesize a charter from the raw text: use existing `charter_dialogue` machinery in single-pass mode (system prompt: "Given this job description, produce a concise charter under 500 words capturing role, responsibilities, success metrics, communication style") — reuse the `ProviderClient` interface from `mission-genesis-llm-classifier` (in flight) so this brief doesn't pull all of argo-provider in.
3. Pass the synthesized charter through the existing Genesis pipeline (classifier → manifest → seed actions).
4. Return `GenesisDraft { synthesized_charter: String, archetype: Archetype, manifest: WorkerManifest, eval_seed: Vec<EvalCase> }` — exactly the same shape Genesis already produces from a typed charter.

CLI: `argo genesis from-job-description <path-or-url>` invokes the pipeline + prints the draft to stdout (JSON for `--json`).

This is NOT in scope: streaming PDF for large files (v1 = read whole file); DOCX/RTF (PDF + text + URL only for v1); multi-turn refinement on the synthesized charter (single-pass; operator can edit + re-run `argo genesis from-charter` afterward if needed); image-only PDFs (text-extractable PDFs only — OCR is a separate brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-genesis/src/job_description.rs` (NEW, ≤ 350 LOC) — `JobDescInput` enum, `genesis_from_job_description` async fn, PDF / URL / Text dispatch.
- `crates/argo-genesis/src/job_description_tests.rs` (NEW, sibling) — 6 unit tests: pdf-input-extracts-text; text-input-passes-through; url-input-fetches; empty-text-errors; pdf-image-only-errors-with-hint; synthesize-charter-falls-through-to-existing-pipeline.
- `crates/argo-genesis/src/lib.rs` — narrowly scoped: ONE `pub mod job_description;` line + re-export `JobDescInput` and `genesis_from_job_description` at crate root.
- `crates/argo-cli/src/handlers/genesis.rs` (or wherever Genesis CLI lives — verify in first 5min via `rg -l "fn cmd_genesis" crates/argo-cli/src/`) — narrowly scoped: ADD `argo genesis from-job-description <path-or-url> [--json]` subcommand. Reuse `argo genesis from-charter` output formatting.
- `crates/argo-cli/tests/genesis_jobdesc_smoke.rs` (NEW, ≤ 150 LOC) — integration smoke: embed a known-text 1-page PDF as test fixture; assert pipeline produces a non-empty manifest.
- `crates/argo-genesis/Cargo.toml` — narrowly scoped: confirm `argo-executor-document` is a dependency (or add). Reqwest likely already present via workspace; verify.

## Files you must NOT touch

- `crates/argo-executor-document/src/pdf.rs` — PDF extractor is upstream; reuse only
- `crates/argo-genesis/src/charter_dialogue.rs` — dialogue machinery is upstream; reuse only
- `crates/argo-genesis/src/archetype.rs` — classifier is upstream; reuse only
- Any non-genesis/non-cli crate
- `crates/argo-store/migrations/**` — no schema change (Genesis is store-less; manifest is returned, not persisted in this brief)

## Acceptance criteria

- [ ] `genesis_from_job_description(JobDescInput) -> ArgoResult<GenesisDraft>` exists, async
- [ ] PDF path uses `argo-executor-document::pdf::extract_text`; falls back gracefully on image-only PDFs with explicit error
- [ ] URL path uses reqwest GET with a 30s timeout and a 5MB body cap
- [ ] Text path is identity
- [ ] After text extraction, single-pass charter synthesis via `ProviderClient` interface (reuse from mission-genesis-llm-classifier if it lands first; else stub with a `TODO: pending classifier brief` and fall back to identity)
- [ ] After synthesis, output is byte-identical to running `argo genesis from-charter <synthesized>` (the rest of the pipeline must NOT branch)
- [ ] CLI `argo genesis from-job-description <input>` works; `--json` flag outputs the full `GenesisDraft` as JSON
- [ ] 6 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-genesis -p argo-cli` clean
- [ ] `cargo clippy -p argo-genesis -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-genesis -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-genesis-job-description-ingest`
- PR title: `genesis: ingest job description PDF/text/url → manifest (mission-genesis-job-description-ingest)`

## Notes for the implementer

- The 5MB URL body cap is a safety guardrail — operators pasting in 50MB PDFs over flaky wifi should fail fast with a clear error, not hang the CLI.
- Single-pass charter synthesis is intentional v1 scope. Multi-turn refinement of a job-description-derived charter is a future brief — for v1, operators who want refinement can run `argo genesis from-job-description` to get a draft, copy the synthesized charter into a file, edit, then run `argo genesis from-charter <edited>`.
- The `image-only PDF` failure mode is real: extraction returns empty/whitespace text. Detect this BEFORE invoking the LLM — emit `ArgoError::Validation("PDF appears to be image-only; OCR not supported in v1. Try converting to text first.")`.
- Don't persist anything in this brief. Returning the `GenesisDraft` and letting the existing `argo genesis save` CLI flow handle persistence keeps the surface narrow.
- Pull the `ProviderClient` trait from `mission-genesis-llm-classifier` (#58, in flight) rather than redefining — if that brief hasn't landed, file a follow-up note in the PR and stub with identity-fn for charter synthesis until it does.
