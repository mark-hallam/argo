"use client";

import { useCallback, useRef, useState } from "react";
import { CheckCircle2, FileText, Loader2, UploadCloud, XCircle } from "lucide-react";
import {
  ingestOnboardingDocuments,
  readFileForIngest,
  wikiUrlDoc,
  INGEST_PHASES,
  ONBOARDING_ACCEPT,
  ONBOARDING_IMAGE_ACCEPT,
  ONBOARDING_MAX_FILE_BYTES,
  ONBOARDING_MAX_FILES,
  type IngestPhase,
  type OnboardingDocInput,
} from "@/lib/api/genesis-ingest";
import { ImageCaptionStage, type ImageCaptionStageHandle } from "./ImageCaptionStage";
import { WikiImportSources } from "./WikiImportSources";

/** Whether a picked file is an image (mirrors `readFileForIngest`). Images
 *  route through the caption-staging row first — a captionless image is rejected. */
function isImageFile(file: File): boolean {
  return file.type.startsWith("image/") || /\.(png|jpe?g|gif|webp)$/i.test(file.name);
}

/**
 * Genesis onboarding document drop-zone (Phase 2 — document upload; memory
 * `genesis-is-ai-teammate-onboarding-studio`).
 *
 * Drop SOPs / wikis / process docs (PDF / Markdown / DOCX / TXT / screenshots)
 * onto the drafted teammate, or paste a wiki / docs URL (fetched server-side via
 * the SSRF-safe fetcher). Each item shows a 4-phase progress bar (uploaded →
 * parsed → chunked → embedded into memory) and is ingested tenant-scoped so the
 * teammate boots already knowing the material. Real backend only — no fixture fallback
 * (operator memory `no-theater-in-demo`): the documents the camera sees uploaded
 * ARE the ones the teammate drills into.
 *
 * Consumes ONLY the locked brand tokens from `globals.css` (purple→PINK
 * gradient, Inter, `--brand-ink`) — no color literals, no new design (DEC-027).
 */

export interface DocumentUploaderProps {
  /** The drafted teammate's `vertical_id` (from `vertical_ready`), or `null`
   *  before the role is described — uploads are disabled until it exists. */
  verticalId: string | null;
  /** Disabled while a generation / hire is in flight. */
  disabled?: boolean;
  /** Notified with the ingested document labels after a successful batch, so
   *  the parent can surface "your teammate has read: …" on the review screen. */
  onIngested?(labels: string[]): void;
}

/** One queued file's display state. */
interface FileState {
  filename: string;
  /** Highest phase reached, or `rejected`. */
  phase: IngestPhase | "queued" | "rejected";
  chunks?: number;
  reason?: string;
}

const PHASE_LABEL: Record<IngestPhase, string> = {
  uploaded: "Uploaded",
  parsed: "Parsed",
  chunked: "Chunked",
  embedded: "In memory",
};

export function DocumentUploader({
  verticalId,
  disabled,
  onIngested,
}: DocumentUploaderProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const stageRef = useRef<ImageCaptionStageHandle | null>(null);
  const [files, setFiles] = useState<FileState[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);

  const blocked = disabled || busy || !verticalId;

  // Shared submit path: show each doc as in-flight, POST the batch, then
  // reconcile per-file results (used by both file uploads and URL intake).
  const submitDocs = useCallback(
    async (docs: OnboardingDocInput[]) => {
      if (!verticalId || docs.length === 0) return;
      setFiles((prev) => [
        ...prev,
        ...docs.map((d) => ({ filename: d.filename, phase: "uploaded" as const })),
      ]);
      setBusy(true);
      try {
        const res = await ingestOnboardingDocuments(verticalId, docs);
        setFiles((prev) => {
          const next = [...prev];
          for (const r of res.results) {
            // Update the most recent matching row that is still in flight.
            for (let i = next.length - 1; i >= 0; i -= 1) {
              if (next[i].filename === r.filename && next[i].phase === "uploaded") {
                next[i] =
                  r.status === "ingested"
                    ? { filename: r.filename, phase: "embedded", chunks: r.chunks }
                    : { filename: r.filename, phase: "rejected", reason: r.reason };
                break;
              }
            }
          }
          return next;
        });
        if (res.ingested > 0) onIngested?.(res.labels);
      } catch {
        setError("Couldn't reach the ingest service. Please try again.");
        // Roll the just-added in-flight rows back to a rejected state.
        setFiles((prev) =>
          prev.map((f) =>
            f.phase === "uploaded"
              ? { ...f, phase: "rejected", reason: "Upload failed — try again." }
              : f,
          ),
        );
      } finally {
        setBusy(false);
        if (inputRef.current) inputRef.current.value = "";
      }
    },
    [verticalId, onIngested],
  );

  const ingest = useCallback(
    async (picked: File[]) => {
      setError(null);
      if (!verticalId) {
        setError("Describe the role first — then drop your documents here.");
        return;
      }
      if (picked.length === 0) return;
      if (picked.length > ONBOARDING_MAX_FILES) {
        setError(`Up to ${ONBOARDING_MAX_FILES} files per session.`);
        return;
      }
      const oversize = picked.find((f) => f.size > ONBOARDING_MAX_FILE_BYTES);
      if (oversize) {
        setError(`"${oversize.name}" is over 50 MB — please attach a smaller file.`);
        return;
      }
      // Images need an operator caption (the gradeable text — no fake OCR), so
      // they route through the staging rows; text / PDF / DOCX ingest directly.
      const images = picked.filter(isImageFile);
      if (images.length > 0) stageRef.current?.addImages(images);
      const rest = picked.filter((f) => !isImageFile(f));
      if (rest.length === 0) return;
      const docs = await Promise.all(rest.map((f) => readFileForIngest(f)));
      await submitDocs(docs);
    },
    [verticalId, submitDocs],
  );

  // A staged image confirmed with a caption: read it WITH the caption, then submit.
  const ingestImage = useCallback(
    async (file: File, caption: string) => {
      setError(null);
      await submitDocs([await readFileForIngest(file, caption)]);
    },
    [submitDocs],
  );

  // Submit a pasted URL through the existing `wiki_url` ingest path. Returns
  // true when accepted (so `WikiImportSources` clears its input), false when
  // validation rejected it. Source chips only adjust the input's guidance copy;
  // every source funnels into this one `format:"wiki_url"` POST.
  const ingestUrl = useCallback(
    async (raw: string): Promise<boolean> => {
      setError(null);
      if (!verticalId) {
        setError("Describe the role first — then add a wiki link.");
        return false;
      }
      const trimmed = raw.trim();
      if (!trimmed) return false;
      if (!/^https?:\/\//i.test(trimmed)) {
        setError("Enter a full http(s) link to a wiki or docs page.");
        return false;
      }
      await submitDocs([wikiUrlDoc(trimmed)]);
      return true;
    },
    [verticalId, submitDocs],
  );

  const onDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setDragOver(false);
      if (blocked) return;
      void ingest(Array.from(e.dataTransfer.files));
    },
    [blocked, ingest],
  );

  return (
    <div className="doc-up" data-testid="genesis-document-uploader">
      <style jsx>{`
        .doc-up {
          margin: 18px auto 0;
          max-width: 560px;
          font-family: var(--font-sans);
        }
        .doc-zone {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 8px;
          padding: 22px 18px;
          border-radius: 16px;
          border: 1.5px dashed color-mix(in srgb, var(--brand-primary) 38%, transparent);
          background: linear-gradient(
            135deg,
            color-mix(in srgb, var(--brand-primary) 5%, #fff),
            color-mix(in srgb, var(--brand-pink) 5%, #fff)
          );
          color: var(--brand-ink);
          cursor: pointer;
          transition: border-color 0.15s ease, background 0.15s ease, opacity 0.15s ease;
          text-align: center;
        }
        .doc-zone.drag {
          border-color: var(--brand-pink);
          background: linear-gradient(
            135deg,
            color-mix(in srgb, var(--brand-primary) 12%, #fff),
            color-mix(in srgb, var(--brand-pink) 12%, #fff)
          );
        }
        .doc-zone.blocked {
          opacity: 0.55;
          cursor: not-allowed;
        }
        .doc-title {
          font-size: 15px;
          font-weight: 700;
        }
        .doc-sub {
          font-size: 12.5px;
          line-height: 1.45;
          color: color-mix(in srgb, var(--brand-ink) 62%, transparent);
          max-width: 420px;
        }
        .doc-icon {
          color: var(--brand-primary);
        }
        .doc-list {
          list-style: none;
          margin: 12px 0 0;
          padding: 0;
          display: flex;
          flex-direction: column;
          gap: 8px;
        }
        .doc-item {
          display: flex;
          align-items: center;
          gap: 10px;
          padding: 9px 12px;
          border-radius: 12px;
          border: 1px solid color-mix(in srgb, var(--brand-ink) 10%, transparent);
          background: #fff;
        }
        .doc-name {
          flex: 1;
          font-size: 13.5px;
          font-weight: 600;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
        .doc-phases {
          display: flex;
          gap: 4px;
        }
        .doc-dot {
          width: 30px;
          height: 5px;
          border-radius: 999px;
          background: color-mix(in srgb, var(--brand-ink) 12%, transparent);
          transition: background 0.25s ease;
        }
        .doc-dot.on {
          background: linear-gradient(90deg, var(--brand-primary), var(--brand-pink));
        }
        .doc-phase-label {
          font-size: 11.5px;
          font-weight: 600;
          min-width: 64px;
          text-align: right;
          color: color-mix(in srgb, var(--brand-ink) 70%, transparent);
        }
        .doc-phase-label.bad {
          color: var(--brand-coral, #ff6b6b);
        }
        .doc-ok {
          color: var(--brand-primary);
        }
        .doc-bad {
          color: var(--brand-coral, #ff6b6b);
        }
        .doc-reason {
          font-size: 11.5px;
          color: var(--brand-coral, #ff6b6b);
        }
        .doc-error {
          margin-top: 8px;
          color: var(--brand-coral, #ff6b6b);
          font-size: 12.5px;
          font-weight: 600;
        }
        .spin {
          animation: doc-spin 0.9s linear infinite;
        }
        @keyframes doc-spin {
          to {
            transform: rotate(360deg);
          }
        }
      `}</style>

      <input
        ref={inputRef}
        type="file"
        accept={`${ONBOARDING_ACCEPT},${ONBOARDING_IMAGE_ACCEPT}`}
        multiple
        hidden
        data-testid="genesis-document-input"
        onChange={(e) => void ingest(Array.from(e.target.files ?? []))}
      />

      <div
        className={`doc-zone${dragOver ? " drag" : ""}${blocked ? " blocked" : ""}`}
        role="button"
        tabIndex={0}
        data-testid="genesis-document-dropzone"
        onClick={() => {
          if (!blocked) inputRef.current?.click();
        }}
        onKeyDown={(e) => {
          if ((e.key === "Enter" || e.key === " ") && !blocked) inputRef.current?.click();
        }}
        onDragOver={(e) => {
          e.preventDefault();
          if (!blocked) setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={onDrop}
      >
        {busy ? (
          <Loader2 size={26} className="doc-icon spin" aria-hidden />
        ) : (
          <UploadCloud size={26} className="doc-icon" aria-hidden />
        )}
        <span className="doc-title">
          Drop SOPs, wikis, or process docs here
        </span>
        <span className="doc-sub">
          {verticalId
            ? "Your teammate will read them — PDF, Markdown, DOCX, text, or screenshots. They boot already knowing your return policy, escalation matrix, and compliance constraints."
            : "Describe the role first, then drop your documents here."}
        </span>
      </div>

      <WikiImportSources disabled={blocked} onAddLink={ingestUrl} />

      <ImageCaptionStage
        ref={stageRef}
        disabled={blocked}
        onIngest={(file, caption) => void ingestImage(file, caption)}
      />

      {files.length > 0 ? (
        <ul className="doc-list" data-testid="genesis-document-list">
          {files.map((f, idx) => (
            <li className="doc-item" key={`${f.filename}-${idx}`}>
              {f.phase === "rejected" ? (
                <XCircle size={16} className="doc-bad" aria-hidden />
              ) : f.phase === "embedded" ? (
                <CheckCircle2 size={16} className="doc-ok" aria-hidden />
              ) : (
                <FileText size={16} className="doc-icon" aria-hidden />
              )}
              <span className="doc-name" title={f.filename}>
                {f.filename}
              </span>
              {f.phase === "rejected" ? (
                <span className="doc-reason" title={f.reason}>
                  {f.reason ?? "Skipped"}
                </span>
              ) : (
                <>
                  <span className="doc-phases" aria-hidden>
                    {INGEST_PHASES.map((p) => (
                      <span
                        key={p}
                        className={`doc-dot${phaseReached(f.phase, p) ? " on" : ""}`}
                      />
                    ))}
                  </span>
                  <span className="doc-phase-label">
                    {f.phase === "embedded"
                      ? `${PHASE_LABEL.embedded}${f.chunks ? ` · ${f.chunks} chunks` : ""}`
                      : PHASE_LABEL[f.phase as IngestPhase] ?? "Uploading"}
                  </span>
                </>
              )}
            </li>
          ))}
        </ul>
      ) : null}

      {error ? (
        <div className="doc-error" role="alert" data-testid="genesis-document-error">
          {error}
        </div>
      ) : null}
    </div>
  );
}

/** Whether `phase` has reached (or passed) `target` in the 4-phase order. */
function phaseReached(phase: FileState["phase"], target: IngestPhase): boolean {
  if (phase === "queued" || phase === "rejected") return false;
  return INGEST_PHASES.indexOf(phase) >= INGEST_PHASES.indexOf(target);
}
