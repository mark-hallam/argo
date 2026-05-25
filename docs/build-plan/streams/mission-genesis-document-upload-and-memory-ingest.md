# mission-genesis-document-upload-and-memory-ingest

**Owner**: claude.
**Authored against**: origin/main `a85963b34` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-document-upload-and-memory-ingest`.
**Depends on**: `mission-genesis-gateway-route-streaming-api` + `mission-genesis-spa-page-describe-and-watch` MERGED.

🔴 **P0 MISSION CLAIM 3 — Phase 2 Onboarding Studio — company-doc upload during onboarding.**

Per memory [[genesis-is-ai-teammate-onboarding-studio]], a real teammate learns the company by reading SOPs, wikis, and internal docs. This brief adds the upload path: operator drags PDFs / Markdown / DOCX into `/genesis`, the files are parsed, and their content is ingested into the new worker's memory schema at hire time. The worker boots already-knowing the material.

The investor moment: *"I uploaded our SOPs PDF — the worker already knows our return policy, our escalation matrix, and our compliance constraints."*

## Scope

1. **SPA surface**: on `/genesis`, add a drop-zone below the textarea: "Drop SOPs, wikis, or process docs here — your teammate will read them." Accepts PDF, Markdown (.md), DOCX, plain text. Multiple-file support.
2. **Upload pipeline**: files POST to `/api/v1/genesis/ingest-document` (one per file or batch endpoint — worker's call). Backend converts to plain text via existing `argo-executor-document` family (PDF → text, DOCX → text, MD passthrough) — these crates already exist per `eval-document-executor-cross-tenant-scoping` audit context.
3. **Per-document progress**: each upload shows a 4-phase progress bar (uploaded / parsed / chunked / embedded-into-memory). SSE-streamed back through the same `/api/v1/genesis` channel so the parent Genesis progress strip stays cohesive.
4. **Memory schema integration**: extracted text is chunked + (optionally) embedded + stored in the worker's `memory` table scoped to a new namespace like `onboarding_documents`. The new worker's first-turn system prompt includes a retrieval-augmented preamble: *"You have access to the following onboarding documents your manager uploaded: [list]."*
5. **Cross-tenant**: documents are tenant-scoped. Tenant A's uploaded SOP never leaks to tenant B's worker. Includes negative test.
6. **Size caps**: 50 MB per file, 200 MB total, 50 files max per Genesis session. Document larger than caps → friendly error.

## Acceptance criteria

- [ ] Drop-zone accepts PDF / MD / DOCX / TXT; files queue with per-file progress.
- [ ] After ingest, the new worker's memory contains chunks tagged with `onboarding_documents` namespace + source filename.
- [ ] First worker turn after Hire: ask the worker "What did you learn from the uploaded docs?" — response cites specific content from the uploaded files (verifies retrieval works).
- [ ] Cross-tenant: tenant A's uploaded SOPs are NOT in tenant B's `audit/memory/list` output.
- [ ] PDF parsing handles a 10-page real SOP cleanly. Failure mode for corrupted files: clear "Couldn't parse foo.pdf — skipped" message; remaining files still ingest.
- [ ] Function-side: the documents the camera SEES uploaded MUST be the ones the worker drills into; no fixture fallback.

## May-touch

- `web/components/skycot/genesis/DocumentUploader.tsx` (NEW)
- `web/lib/api/genesis-ingest.ts` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_genesis_ingest.rs` (NEW — multipart upload + dispatch to argo-executor-document)
- `crates/argo-genesis/src/onboarding_documents.rs` (NEW — chunking + memory writer)
- `crates/argo-memory/src/onboarding_namespace.rs` (NEW or extend existing — namespace + tenant binding)
- Tests for cross-tenant + parse-failure + size-cap

## Out of scope

- Real-time collaborative upload (single-operator session for v1.0)
- OCR for scanned PDFs (text PDFs only for v1.0)
- Document update flow post-Hire (separate brief if needed)
- Wiki / Slack imports — separate brief: `mission-genesis-integration-import-slack-notion-confluence`

## Effort

~400-700 LOC (frontend uploader + gateway multipart + chunking pipeline + memory writes + 2-3 cross-tenant tests). ~5-8 hours.
