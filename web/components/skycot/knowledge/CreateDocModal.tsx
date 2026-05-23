"use client";

import { useState } from "react";
import type {
  CreateKnowledgeDocRequest,
  KnowledgeCategory,
} from "@/lib/api/workplace-knowledge";
import { CATEGORY_META } from "./category-meta";

interface Props {
  isSubmitting: boolean;
  submitError: string | null;
  onCancel: () => void;
  onSubmit: (req: CreateKnowledgeDocRequest) => void;
}

/** New-doc modal. Slug autocompletes from the title until the user
 *  edits it. Form validates title + slug + body before enabling Save. */
export function CreateDocModal({
  isSubmitting,
  submitError,
  onCancel,
  onSubmit,
}: Props) {
  const [title, setTitle] = useState("");
  const [slug, setSlug] = useState("");
  const [slugDirty, setSlugDirty] = useState(false);
  const [category, setCategory] = useState<KnowledgeCategory>("brand");
  const [body, setBody] = useState("");

  const derivedSlug = slugDirty ? slug : slugify(title);
  const ready =
    !isSubmitting &&
    title.trim().length > 0 &&
    derivedSlug.trim().length > 0 &&
    /^[a-z0-9-]+$/.test(derivedSlug) &&
    body.trim().length > 0;

  return (
    <div
      className="knowledge-modal-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="knowledge-modal-title"
      onClick={onCancel}
    >
      <div className="knowledge-modal" onClick={(e) => e.stopPropagation()}>
        <h2 id="knowledge-modal-title">New workplace doc</h2>

        <div className="knowledge-form-field">
          <label htmlFor="knowledge-modal-title-input">Title</label>
          <input
            id="knowledge-modal-title-input"
            data-testid="knowledge-modal-title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Brand voice"
          />
        </div>

        <div className="knowledge-form-field">
          <label htmlFor="knowledge-modal-slug-input">Slug</label>
          <input
            id="knowledge-modal-slug-input"
            data-testid="knowledge-modal-slug"
            value={derivedSlug}
            onChange={(e) => {
              setSlug(e.target.value);
              setSlugDirty(true);
            }}
            placeholder="brand-voice"
          />
          {derivedSlug && !/^[a-z0-9-]+$/.test(derivedSlug) ? (
            <span style={{ fontSize: 11, color: "#b91c1c" }}>
              Slug must be lowercase letters, digits, or dashes.
            </span>
          ) : null}
        </div>

        <div className="knowledge-form-field">
          <label htmlFor="knowledge-modal-category-input">Category</label>
          <select
            id="knowledge-modal-category-input"
            data-testid="knowledge-modal-category"
            value={category}
            onChange={(e) => setCategory(e.target.value as KnowledgeCategory)}
          >
            {CATEGORY_META.map((c) => (
              <option key={c.id} value={c.id}>
                {c.label}
              </option>
            ))}
          </select>
        </div>

        <div className="knowledge-form-field">
          <label htmlFor="knowledge-modal-body-input">Markdown body</label>
          <textarea
            id="knowledge-modal-body-input"
            data-testid="knowledge-modal-body"
            value={body}
            onChange={(e) => setBody(e.target.value)}
            placeholder="# My new doc..."
          />
        </div>

        {submitError ? (
          <div className="knowledge-error-banner" role="alert">
            {submitError}
          </div>
        ) : null}

        <div className="knowledge-modal-actions">
          <button
            type="button"
            className="knowledge-modal-cancel"
            onClick={onCancel}
            disabled={isSubmitting}
          >
            Cancel
          </button>
          <button
            type="button"
            className="knowledge-save-btn"
            onClick={() =>
              onSubmit({
                title: title.trim(),
                slug: derivedSlug.trim(),
                category,
                body_markdown: body,
              })
            }
            disabled={!ready}
            data-testid="knowledge-modal-submit"
          >
            {isSubmitting ? "Creating…" : "Create doc"}
          </button>
        </div>
      </div>
    </div>
  );
}

/** Build a kebab-case slug from a title. Lossy on purpose so the user
 *  is encouraged to refine it manually for short, memorable URLs. */
export function slugify(title: string): string {
  return title
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9-\s]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}
