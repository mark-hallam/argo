"use client";

import { forwardRef, useImperativeHandle, useState } from "react";
import { ImagePlus, X } from "lucide-react";

/**
 * Genesis image-caption staging row (sibling of `DocumentUploader`).
 *
 * The multi-modal Genesis backend grades an uploaded screenshot over an
 * operator-typed caption (no fake OCR — a captionless image is REJECTED). The
 * drop-zone can't send a file straight through, so dropped/picked images land
 * here first: each shows an inline caption input and stays un-ingestable until
 * the operator describes it. On confirm the parent reads the file WITH the
 * caption and ingests it; mirrors the backend requirement so the user gets an
 * affordance, not a silent rejection.
 *
 * Consumes ONLY the locked brand tokens from `globals.css` (purple→PINK
 * gradient, Inter, `--brand-ink`) — no color literals, no new design (DEC-027).
 */

export interface ImageCaptionStageHandle {
  /** Queue freshly picked image files for captioning. */
  addImages(files: File[]): void;
}

export interface ImageCaptionStageProps {
  /** Disabled while a generation / hire / ingest is in flight. */
  disabled?: boolean;
  /** Called when the operator confirms a captioned image (caption non-empty). */
  onIngest(file: File, caption: string): void;
}

interface StagedImage {
  file: File;
  caption: string;
}

export const ImageCaptionStage = forwardRef<ImageCaptionStageHandle, ImageCaptionStageProps>(
  function ImageCaptionStage({ disabled, onIngest }, ref) {
    const [staged, setStaged] = useState<StagedImage[]>([]);

    useImperativeHandle(
      ref,
      () => ({
        addImages(files: File[]) {
          if (files.length === 0) return;
          setStaged((prev) => [...prev, ...files.map((file) => ({ file, caption: "" }))]);
        },
      }),
      [],
    );

    const setCaption = (idx: number, caption: string) =>
      setStaged((prev) => prev.map((s, i) => (i === idx ? { ...s, caption } : s)));

    const remove = (idx: number) => setStaged((prev) => prev.filter((_, i) => i !== idx));

    const confirm = (idx: number) => {
      const item = staged[idx];
      const caption = item?.caption.trim() ?? "";
      if (!item || caption.length === 0) return;
      remove(idx);
      onIngest(item.file, caption);
    };

    if (staged.length === 0) return null;

    return (
      <ul className="img-stage" data-testid="genesis-image-stage">
        <style jsx>{`
          .img-stage {
            list-style: none;
            margin: 12px 0 0;
            padding: 0;
            display: flex;
            flex-direction: column;
            gap: 8px;
          }
          .img-row {
            display: flex;
            align-items: center;
            gap: 8px;
            padding: 9px 12px;
            border-radius: 12px;
            border: 1px solid color-mix(in srgb, var(--brand-primary) 30%, transparent);
            background: linear-gradient(
              135deg,
              color-mix(in srgb, var(--brand-primary) 5%, #fff),
              color-mix(in srgb, var(--brand-pink) 5%, #fff)
            );
            font-family: var(--font-sans);
          }
          .img-icon {
            color: var(--brand-primary);
            flex-shrink: 0;
          }
          .img-name {
            font-size: 13px;
            font-weight: 600;
            color: var(--brand-ink);
            max-width: 120px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
          }
          .img-caption {
            flex: 1;
            padding: 7px 10px;
            border-radius: 10px;
            border: 1px solid color-mix(in srgb, var(--brand-ink) 14%, transparent);
            font-size: 13px;
            font-family: var(--font-sans);
            color: var(--brand-ink);
            background: #fff;
          }
          .img-add {
            padding: 8px 14px;
            border-radius: 10px;
            border: none;
            font-size: 12.5px;
            font-weight: 700;
            color: #fff;
            cursor: pointer;
            background: linear-gradient(90deg, var(--brand-primary), var(--brand-pink));
            transition: opacity 0.15s ease;
            flex-shrink: 0;
          }
          .img-add:disabled {
            opacity: 0.5;
            cursor: not-allowed;
          }
          .img-remove {
            border: none;
            background: none;
            cursor: pointer;
            color: color-mix(in srgb, var(--brand-ink) 50%, transparent);
            display: flex;
            align-items: center;
            padding: 2px;
            flex-shrink: 0;
          }
        `}</style>
        {staged.map((s, idx) => (
          <li className="img-row" key={`${s.file.name}-${idx}`}>
            <ImagePlus size={16} className="img-icon" aria-hidden />
            <span className="img-name" title={s.file.name}>
              {s.file.name}
            </span>
            <input
              className="img-caption"
              type="text"
              placeholder="Describe what this screenshot shows…"
              aria-label={`Caption for ${s.file.name}`}
              data-testid="genesis-image-caption-input"
              value={s.caption}
              disabled={disabled}
              onChange={(e) => setCaption(idx, e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  confirm(idx);
                }
              }}
            />
            <button
              type="button"
              className="img-add"
              data-testid="genesis-image-caption-add"
              disabled={disabled || s.caption.trim().length === 0}
              onClick={() => confirm(idx)}
            >
              Add to memory
            </button>
            <button
              type="button"
              className="img-remove"
              aria-label={`Remove ${s.file.name}`}
              data-testid="genesis-image-caption-remove"
              onClick={() => remove(idx)}
            >
              <X size={15} aria-hidden />
            </button>
          </li>
        ))}
      </ul>
    );
  },
);
