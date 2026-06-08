/**
 * Genesis onboarding-document ingest client — `POST
 * /api/v1/genesis/ingest-document`.
 *
 * After the teammate is drafted (a `vertical_ready` event arrives with a
 * `vertical_id`), the operator can drop SOPs / wikis / process docs onto it.
 * Each file is base64-encoded (PDF) or sent as text (Markdown / plain text),
 * the gateway parses + chunks + persists them tenant-scoped into the teammate's
 * memory, and the teammate boots already knowing the material.
 *
 * Real backend only — no stub fallback (operator memory `no-theater-in-demo`):
 * the documents the camera sees uploaded ARE the ones ingested.
 */

import { apiPost } from "./client";

/** Accepted upload extensions. PDF goes base64; the rest are sent as text. */
export const ONBOARDING_ACCEPT = ".txt,.md,.markdown,.text,.pdf,.docx";

/** Per-file cap mirrors the gateway's `MAX_FILE_BYTES` (50 MB). */
export const ONBOARDING_MAX_FILE_BYTES = 50 * 1024 * 1024;
/** Combined cap mirrors the gateway's `MAX_TOTAL_BYTES` (200 MB). */
export const ONBOARDING_MAX_TOTAL_BYTES = 200 * 1024 * 1024;
/** File-count cap mirrors the gateway's `MAX_FILES`. */
export const ONBOARDING_MAX_FILES = 50;

/** The four per-file phases the gateway reports on success. */
export const INGEST_PHASES = ["uploaded", "parsed", "chunked", "embedded"] as const;
export type IngestPhase = (typeof INGEST_PHASES)[number];

/** One document in an ingest request.
 *
 * Phase-2 adds three modalities alongside `text` / `pdf_base64`:
 * - `image_base64` — `content` is base64 image bytes; `caption` is the
 *   operator's description (the gradeable text — no fake OCR).
 * - `voice` — `transcript` carries text the live `/voice/stt` loop produced.
 * - `wiki_url` — `content` is a URL, fetched server-side via the SSRF-safe
 *   fetcher.
 */
export interface OnboardingDocInput {
  filename: string;
  format: "text" | "pdf_base64" | "image_base64" | "voice" | "wiki_url";
  content: string;
  /** Operator description for an `image_base64` document. */
  caption?: string;
  /** Pre-transcribed text for a `voice` document (from `/voice/stt`). */
  transcript?: string;
}

/** Accepted image extensions for screenshot / diagram onboarding. */
export const ONBOARDING_IMAGE_ACCEPT = ".png,.jpg,.jpeg,.gif,.webp";

/** Per-file ingest outcome returned by the gateway. */
export interface IngestFileResult {
  filename: string;
  status: "ingested" | "rejected";
  chunks?: number;
  reason?: string;
  phases?: IngestPhase[];
}

/** The gateway's batch ingest response. */
export interface IngestResponse {
  results: IngestFileResult[];
  ingested: number;
  rejected: number;
  labels: string[];
  namespace: string;
}

/**
 * Ingest a batch of onboarding documents into the drafted teammate's memory.
 *
 * @param verticalId the `vertical_id` from the `vertical_ready` event.
 * @param documents the files to ingest (already read to text/base64).
 */
export async function ingestOnboardingDocuments(
  verticalId: string,
  documents: OnboardingDocInput[],
): Promise<IngestResponse> {
  return apiPost<IngestResponse>("/api/v1/genesis/ingest-document", {
    vertical_id: verticalId,
    documents,
  });
}

/**
 * Read a `File` into the shape the gateway expects — PDF → base64, image →
 * `image_base64` (with an operator `caption`), everything else → UTF-8 text.
 * DOCX is sent as text too (the gateway extracts what text it can; a binary-only
 * DOCX is reported `rejected` and skipped).
 *
 * @param caption optional description, used only for image uploads (the
 *   gradeable text — the gateway skips a captionless image rather than guessing).
 */
export async function readFileForIngest(
  file: File,
  caption?: string,
): Promise<OnboardingDocInput> {
  const name = file.name.toLowerCase();
  const isPdf = file.type === "application/pdf" || name.endsWith(".pdf");
  if (isPdf) {
    return { filename: file.name, format: "pdf_base64", content: await readAsBase64(file) };
  }
  const isImage =
    file.type.startsWith("image/") ||
    /\.(png|jpe?g|gif|webp)$/.test(name);
  if (isImage) {
    return {
      filename: file.name,
      format: "image_base64",
      content: await readAsBase64(file),
      caption: caption?.trim() || "",
    };
  }
  return { filename: file.name, format: "text", content: await file.text() };
}

/**
 * Build a wiki / docs URL ingest document. The gateway fetches it server-side
 * through the SSRF-safe fetcher and ingests the page text.
 */
export function wikiUrlDoc(url: string): OnboardingDocInput {
  const trimmed = url.trim();
  return { filename: trimmed, format: "wiki_url", content: trimmed };
}

/** Read a File as base64 (strips any data-URL prefix). Mirrors `SopUpload`. */
async function readAsBase64(file: File): Promise<string> {
  const buf = await file.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}
