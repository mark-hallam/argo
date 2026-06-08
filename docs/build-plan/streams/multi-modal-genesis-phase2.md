# multi-modal-genesis-phase2 (FINAL SPRINT) — voice + image + wiki-URL onboarding intake

**Owner**: claude.
**Authored against**: origin/main `f491ca8da` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh multi-modal-genesis-phase2`.

## Why (evidence)
Genesis Phase-1 multi-modal onboarding is LIVE for **text docs** but not voice / image
/ wiki-URL:
- `crates/argo-cli/src/handlers/gateway/routes_genesis_ingest.rs` —
  `POST /api/v1/genesis/ingest-document` ingests SOP PDF / Markdown / DOCX / TXT into
  the tenant-scoped onboarding namespace (helpers `extract_sop_pdf_text`,
  `ingest_onboarding_document`, `write_onboarding_documents` at line 40; tenant-bound
  via `genesis_vertical_dir_for_tenant`, lines 32-35 — tenant A cannot write into
  tenant B).
- `crates/argo-cli/src/handlers/gateway/routes_genesis_quiz.rs` —
  `POST /api/v1/genesis/quiz` grades comprehension over `read_onboarding_documents` +
  `answer_quiz_question` (line 40), tenant-isolated (line 36).
- SPA `web/components/skycot/genesis/DocumentUploader.tsx` accepts "PDF / Markdown /
  DOCX / TXT" only (header line 19).

Phase-2 extends the SAME tenant-scoped onboarding namespace to **(a)** voice-narrated
role intake, **(b)** image/screenshot, **(c)** wiki-URL ingest, and grades
comprehension over the added material.

## ⚠️ URL-fetch security (REQUIRED scope — do NOT skip)
A URL fetcher already exists — `crates/argo-genesis/src/job_description.rs:169`
`async fn fetch_url` — but it has ONLY a timeout + body-size cap; it has **NO SSRF
guard**: it would happily GET `http://169.254.169.254/…` (cloud metadata),
`http://localhost`, or RFC-1918 private IPs. Wiki-URL ingest **must not** reuse
`fetch_url` as-is. Add (and route wiki-URL through) an SSRF-safe fetch: reject
non-`http(s)` schemes, resolve the host and **block private / loopback / link-local /
metadata ranges** before connecting, cap redirects, keep the body cap + timeout. This
is the security review the brief calls out — treat it as a hard acceptance gate.

## Grep-before-write (REQUIRED per DRIFT_PREVENTION)
- `rg -n 'pub fn (ingest_onboarding_document|write_onboarding_documents|read_onboarding_documents|extract_sop_pdf_text|answer_quiz_question|suggested_quiz_questions)|pub struct OnboardingDocument' crates/argo-genesis/src` — REUSE these; extend `OnboardingDocument` additively (e.g. a `source_kind` field), do NOT fork a parallel doc type.
- `rg -n 'onboarding_namespace' crates/argo-memory/src/lib.rs` — write all new modalities into the SAME tenant-bound namespace contract.
- `rg -n 'fn fetch_url|URL_TIMEOUT|URL_BODY_CAP_BYTES' crates/argo-genesis/src/job_description.rs` — see what NOT to reuse; build the SSRF-safe fetch alongside it (or harden it — but do not regress the existing job-description path).
- `rg -n 'whisper|transcribe|stt|audio|argo_provider::Transport' crates/argo-voice/src crates/argo-cli/src/handlers/gateway/routes_voice*.rs` — reuse the EXISTING voice STT seam (the live voice loop already transcribes); do NOT add a second STT path.
- `rg -n 'genesis_vertical_dir_for_tenant|genesis_tenant' crates/argo-cli/src/handlers/gateway/routes_genesis_ingest.rs` — every new ingest MUST resolve the caller's own write-tenant the same way.

## Files you may touch
- `crates/argo-cli/src/handlers/gateway/routes_genesis_ingest.rs` (add modality
  branches: voice → transcribe via the existing STT seam → ingest the transcript as an
  onboarding doc; image/screenshot → store + a text descriptor/OCR-if-available;
  wiki-URL → SSRF-safe fetch → ingest). Keep < 500 LOC — split into
  `routes_genesis_ingest_voice.rs` / `routes_genesis_ingest_url.rs` siblings if needed.
- new sibling ingest modules as above.
- `crates/argo-genesis/src/onboarding_documents.rs` (additive `source_kind`/metadata on
  `OnboardingDocument`; keep existing tests green).
- new `crates/argo-genesis/src/safe_fetch.rs` (the SSRF-guarded fetcher) OR harden
  `job_description.rs::fetch_url` without regressing it.
- `crates/argo-genesis/src/sandbox_session.rs` (grade comprehension over the added
  material — reuse `answer_quiz_question`).
- SPA `web/components/skycot/genesis/DocumentUploader.tsx` (+ a small voice/url intake
  control) — accept the new modalities; un-mock under demo-live.
- tests siblings + the relevant genesis vitest.

## Acceptance (truth-bounded + testable)
- An integration test per modality: a voice clip (or a stubbed STT transcript), an
  image, and a wiki-URL each ingest into the tenant onboarding namespace and then a
  quiz question is gradeable over the ADDED material (the new content actually reached
  the namespace `read_onboarding_documents` reads).
- An **SSRF test**: a wiki-URL pointing at `http://127.0.0.1`, `http://169.254.169.254`,
  and an RFC-1918 host is REJECTED before any connection (assert the guard fires);
  `file://` / non-http schemes rejected.
- A **cross-tenant negative test**: tenant A's voice/image/URL ingest can never be read
  by tenant B's quiz (mirror the existing Phase-1 isolation test).
- `cargo test -p argo-genesis -p argo-cli` green; `tsc` + the touched-file vitest green.
- No live model required for the deterministic ingest/grade paths (provider only where
  Phase-1 already used it).

## On ship
Update the FEATURE_MATRIX Genesis row (text-only → multi-modal onboarding Phase-2) with
the commit SHA + routes in the SAME commit, then
`python3 scripts/render-feature-matrix.py`.

## Effort
M (Rust + web). ~350–500 LOC across siblings incl. tests. Additive; no schema change
(onboarding namespace is file-backed per tenant).
