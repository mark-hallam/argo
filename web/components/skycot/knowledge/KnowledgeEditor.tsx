"use client";

import { useState } from "react";
import { Save } from "lucide-react";
import { renderMarkdown } from "./render-markdown";

interface Props {
  body: string;
  onBodyChange: (next: string) => void;
  onSave: (input: { body: string; editReason: string }) => void;
  isSaving: boolean;
  readOnly: boolean;
  /** When true, the save button stays disabled even with edits — used
   *  to lock the form during in-flight requests or in read-only mode. */
  disabled?: boolean;
}

type Tab = "edit" | "preview";

/** Tabbed Edit / Preview markdown editor for the wiki detail page.
 *
 *  Plain textarea + a tiny inline renderer (see `render-markdown.ts`).
 *  The brief's "library choice" gotcha recommends avoiding
 *  `@uiw/react-md-editor` to keep the bundle lean; the brief also
 *  recommends `react-markdown` for the preview, but we keep that out
 *  of the dep list since the wiki surface is small. Vendoring the
 *  ~25KB lib + lockfile churn isn't justified for h1/h2/h3 + lists +
 *  bold/italic/code. */
export function KnowledgeEditor({
  body,
  onBodyChange,
  onSave,
  isSaving,
  readOnly,
  disabled,
}: Props) {
  const [tab, setTab] = useState<Tab>("edit");
  const [editReason, setEditReason] = useState("");

  const tabBtn = (id: Tab, label: string) => (
    <button
      key={id}
      type="button"
      className={
        "knowledge-editor-tab" +
        (tab === id ? " knowledge-editor-tab-active" : "")
      }
      onClick={() => setTab(id)}
      data-testid={`knowledge-editor-tab-${id}`}
    >
      {label}
    </button>
  );

  const handleSave = () => {
    onSave({ body, editReason: editReason.trim() });
  };

  return (
    <div>
      <div className="knowledge-editor-tabs" role="tablist">
        {tabBtn("edit", readOnly ? "View" : "Edit")}
        {tabBtn("preview", "Preview")}
      </div>

      <div className="knowledge-editor-pane">
        {tab === "edit" ? (
          <textarea
            className="knowledge-textarea"
            value={body}
            onChange={(e) => onBodyChange(e.target.value)}
            readOnly={readOnly}
            spellCheck
            data-testid="knowledge-editor-textarea"
            aria-label="Document body markdown"
            placeholder={
              readOnly
                ? "(empty)"
                : "Write your workplace knowledge in markdown. Headings, lists, *italic*, **bold**, and `code` are supported."
            }
          />
        ) : (
          <div
            className="knowledge-preview"
            data-testid="knowledge-editor-preview"
            dangerouslySetInnerHTML={{ __html: renderMarkdown(body) }}
          />
        )}
      </div>

      {readOnly ? null : (
        <div className="knowledge-save-row">
          <input
            type="text"
            className="knowledge-edit-reason"
            placeholder="Why this edit? (optional, ≤200 chars)"
            value={editReason}
            maxLength={200}
            onChange={(e) => setEditReason(e.target.value)}
            data-testid="knowledge-editor-edit-reason"
          />
          <button
            type="button"
            className="knowledge-save-btn"
            onClick={handleSave}
            disabled={isSaving || disabled}
            data-testid="knowledge-editor-save"
          >
            <Save size={14} /> {isSaving ? "Saving…" : "Save"}
          </button>
        </div>
      )}
    </div>
  );
}
