# fix-genesis-image-caption-ui

**Owner**: claude.
**Authored against**: origin/main `502b76bc0` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-genesis-image-caption-ui`.

## Why (conductor adversarial review of #2746, 2026-06-08 — MEDIUM)
The multi-modal Genesis image modality is live + tested at the BACKEND (`routes_genesis_ingest_image.rs`: base64 bytes + an operator caption become the gradeable text, with a SHA-256 provenance descriptor and NO fake OCR — it deliberately REJECTS a captionless image rather than fabricate). But the SPA never collects the caption: `web/components/skycot/genesis/DocumentUploader.tsx` calls `readFileForIngest(f)` with no caption (so `caption: ""`), and the backend rejects an empty caption ("Add a short description…"). Net: every image dropped through the real UI is skipped. The voice + wiki-URL + text/PDF SPA paths are fully wired; only image-via-SPA is dead. FEATURE_MATRIX already marks this honestly (🟡 Partial, image-via-SPA caption-input is a follow-up) — this brief closes it.

## Grep-before-write
- `rg -n 'readFileForIngest|caption|ONBOARDING_IMAGE_ACCEPT|onDrop|ingest\(' web/components/skycot/genesis/DocumentUploader.tsx web/lib/api/genesis-ingest.ts` — find the image-drop handler + the `readFileForIngest(file, caption?)` signature (the vitest already proves it works WHEN a caption is passed).
- Reproduce: drop a `.png` in the `/genesis` DocumentUploader → observe the result row "rejected: Add a short description…".

## Files you may touch
- `web/components/skycot/genesis/DocumentUploader.tsx` — add a caption-input control for image uploads.
- (only if needed) `web/lib/api/genesis-ingest.ts` — already supports the caption param; do not change its contract.

## Fix (consume the LOCKED brand — DEC-027, no new design)
When an image is dropped/picked, collect a short caption from the operator BEFORE calling `readFileForIngest(file, caption)` — e.g. an inline text input on the staged-image row (placeholder "Describe what this screenshot shows…"), or a small modal. Disable/grey the ingest for an image until its caption is non-empty (mirror the backend's requirement so the user gets the affordance, not a silent rejection). Use ONLY `globals.css` brand tokens (purple→PINK gradient, Audiowide/Inter) — no color literals, no new design language (the page direction is locked). Keep the voice/URL/text paths untouched.

## Acceptance (truth-bounded)
1. In the SPA `/genesis` DocumentUploader, an image upload WITH a typed caption ingests successfully (result row "ingested"), and the gradeable text reaches the backend (the existing `/genesis/quiz` can grade over it).
2. An image with an empty caption is blocked at the UI with a clear affordance (not sent + silently rejected).
3. `pnpm -C web exec tsc --noEmit` = 0; touched vitest green (add a case: image with caption → `readFileForIngest` called WITH the caption; without → ingest disabled).
4. No regression to voice / wiki-URL / text-PDF intake.

## On ship (same commit)
Flip the FEATURE_MATRIX Genesis multi-modal row's "🟡 Partial (image-via-SPA…)" qualifier to ✅ Live (image-via-SPA wired) + regen `docs/skycot/feature-matrix.html`. Note it clears the #2746 review MEDIUM.

## Effort
S (one caption-input control + a vitest case).
