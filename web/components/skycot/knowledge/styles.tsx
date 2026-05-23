"use client";

/** Shared styles for the `/settings/knowledge` surface. Single `<style
 *  jsx global>` block keeps the look consistent across the index,
 *  detail, modal, and diff pages without 5 copies of the same rules. */

export function KnowledgeStyles() {
  return (
    <style jsx global>{`
      .knowledge-stage {
        padding: 40px 32px 96px;
        max-width: 1200px;
        margin: 0 auto;
        font-family: "Inter", sans-serif;
      }
      .knowledge-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-end;
        gap: 24px;
        margin-bottom: 28px;
        flex-wrap: wrap;
      }
      .knowledge-eyebrow {
        font-family: var(--font-mono);
        font-size: 11px;
        color: #52525b;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        font-weight: 700;
        margin-bottom: 6px;
      }
      .knowledge-title {
        font-family: "Audiowide", "Inter", sans-serif;
        font-size: 32px;
        font-weight: 400;
        color: #1e1b4b;
        margin: 0 0 6px;
        letter-spacing: -0.01em;
      }
      .knowledge-sub {
        font-size: 14px;
        color: #3f3f46;
        margin: 0;
      }
      .knowledge-layout {
        display: grid;
        grid-template-columns: 220px 1fr;
        gap: 28px;
      }
      @media (max-width: 800px) {
        .knowledge-layout {
          grid-template-columns: 1fr;
        }
      }
      .knowledge-cta {
        display: inline-flex;
        gap: 6px;
        align-items: center;
        padding: 10px 14px;
        border-radius: 10px;
        background: linear-gradient(135deg, #8b5cf6 0%, #ec4899 100%);
        color: white;
        border: none;
        font-weight: 600;
        font-size: 13px;
        cursor: pointer;
      }
      .knowledge-cta:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }
      .knowledge-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
        gap: 14px;
      }
      .knowledge-card {
        background: white;
        border: 1px solid #ece9f5;
        border-radius: 12px;
        padding: 18px 20px;
        text-decoration: none;
        color: inherit;
        transition: all 0.15s;
        display: flex;
        flex-direction: column;
        gap: 10px;
      }
      .knowledge-card:hover {
        border-color: rgba(139, 92, 246, 0.4);
        transform: translateY(-2px);
        box-shadow: 0 6px 12px -4px rgba(139, 92, 246, 0.12);
      }
      .knowledge-card-title {
        font-weight: 600;
        font-size: 15px;
        color: #1e1b4b;
        margin: 0;
      }
      .knowledge-card-meta {
        display: flex;
        gap: 6px;
        align-items: center;
        flex-wrap: wrap;
        font-size: 12px;
        color: #52525b;
      }
      .knowledge-pill {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        padding: 2px 8px;
        border-radius: 999px;
        font-size: 11px;
        font-weight: 600;
        background: #fafaff;
        border: 1px solid #ece9f5;
        color: #52525b;
      }
      .knowledge-pill-version {
        background: rgba(139, 92, 246, 0.08);
        border-color: rgba(139, 92, 246, 0.2);
        color: #6d28d9;
      }
      .knowledge-empty {
        padding: 56px 24px;
        text-align: center;
        background: white;
        border: 1px dashed #ece9f5;
        border-radius: 16px;
        color: #52525b;
        font-size: 14px;
      }
      .knowledge-nav {
        display: flex;
        flex-direction: column;
        gap: 4px;
        background: white;
        border: 1px solid #ece9f5;
        border-radius: 12px;
        padding: 10px;
      }
      .knowledge-nav-item {
        display: flex;
        justify-content: space-between;
        align-items: center;
        background: transparent;
        border: none;
        padding: 9px 12px;
        border-radius: 8px;
        font-size: 13px;
        font-weight: 500;
        color: #3f3f46;
        text-align: left;
        cursor: pointer;
        font-family: inherit;
      }
      .knowledge-nav-item:hover {
        background: #fafaff;
      }
      .knowledge-nav-item-active {
        background: rgba(139, 92, 246, 0.1);
        color: #6d28d9;
      }
      .knowledge-nav-count {
        font-size: 11px;
        color: #52525b;
        font-family: var(--font-mono);
      }
      .knowledge-detail-layout {
        display: grid;
        grid-template-columns: 1fr 320px;
        gap: 28px;
      }
      @media (max-width: 960px) {
        .knowledge-detail-layout {
          grid-template-columns: 1fr;
        }
      }
      .knowledge-editor-tabs {
        display: flex;
        gap: 6px;
        margin-bottom: 12px;
      }
      .knowledge-editor-tab {
        padding: 8px 14px;
        border-radius: 8px;
        background: transparent;
        border: 1px solid #ece9f5;
        color: #52525b;
        font-size: 13px;
        font-weight: 600;
        cursor: pointer;
        font-family: inherit;
      }
      .knowledge-editor-tab-active {
        background: #1e1b4b;
        color: white;
        border-color: #1e1b4b;
      }
      .knowledge-editor-pane {
        background: white;
        border: 1px solid #ece9f5;
        border-radius: 12px;
        padding: 18px;
        min-height: 360px;
      }
      .knowledge-textarea {
        width: 100%;
        min-height: 340px;
        font-family: var(--font-mono);
        font-size: 13px;
        line-height: 1.55;
        border: none;
        outline: none;
        resize: vertical;
        color: #1e1b4b;
        background: transparent;
      }
      .knowledge-textarea[readonly] {
        color: #3f3f46;
        cursor: default;
      }
      .knowledge-preview h1 {
        font-size: 22px;
        font-weight: 700;
        margin: 12px 0 8px;
      }
      .knowledge-preview h2 {
        font-size: 18px;
        font-weight: 700;
        margin: 18px 0 6px;
      }
      .knowledge-preview h3 {
        font-size: 15px;
        font-weight: 700;
        margin: 14px 0 4px;
      }
      .knowledge-preview p {
        font-size: 14px;
        margin: 8px 0;
        line-height: 1.55;
      }
      .knowledge-preview ul,
      .knowledge-preview ol {
        padding-left: 22px;
        font-size: 14px;
      }
      .knowledge-preview code {
        background: #fafaff;
        border-radius: 4px;
        padding: 1px 5px;
        font-size: 12px;
      }
      .knowledge-preview pre {
        background: #fafaff;
        border: 1px solid #ece9f5;
        border-radius: 8px;
        padding: 10px;
        overflow-x: auto;
        font-size: 12px;
      }
      .knowledge-save-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-top: 12px;
        gap: 12px;
        flex-wrap: wrap;
      }
      .knowledge-edit-reason {
        flex: 1;
        min-width: 240px;
        font-family: inherit;
        font-size: 13px;
        padding: 8px 10px;
        border-radius: 8px;
        border: 1px solid #ece9f5;
        background: white;
        color: #1e1b4b;
      }
      .knowledge-save-btn {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 9px 16px;
        border-radius: 8px;
        background: #1e1b4b;
        color: white;
        font-weight: 600;
        font-size: 13px;
        border: none;
        cursor: pointer;
      }
      .knowledge-save-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }
      .knowledge-delete-btn {
        margin-top: 32px;
        padding: 9px 14px;
        background: white;
        color: #b91c1c;
        border: 1px solid rgba(185, 28, 28, 0.25);
        border-radius: 8px;
        font-weight: 600;
        font-size: 12px;
        cursor: pointer;
      }
      .knowledge-version-sidebar {
        background: white;
        border: 1px solid #ece9f5;
        border-radius: 12px;
        padding: 16px;
        max-height: 70vh;
        overflow-y: auto;
      }
      .knowledge-version-sidebar h3 {
        margin: 0 0 12px;
        font-size: 12px;
        font-weight: 700;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: #52525b;
      }
      .knowledge-version-row {
        display: block;
        text-decoration: none;
        color: inherit;
        padding: 10px;
        border-radius: 8px;
        border: 1px solid transparent;
        margin-bottom: 6px;
        font-size: 12px;
      }
      .knowledge-version-row:hover {
        border-color: #ece9f5;
        background: #fafaff;
      }
      .knowledge-version-hash {
        font-family: var(--font-mono);
        font-size: 11px;
        color: #6d28d9;
        background: rgba(139, 92, 246, 0.08);
        padding: 1px 6px;
        border-radius: 4px;
        cursor: pointer;
        border: none;
      }
      .knowledge-modal-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(30, 27, 75, 0.5);
        display: grid;
        place-items: center;
        z-index: 50;
      }
      .knowledge-modal {
        background: white;
        border-radius: 16px;
        padding: 28px;
        max-width: 520px;
        width: calc(100% - 32px);
        max-height: 90vh;
        overflow-y: auto;
      }
      .knowledge-modal h2 {
        margin: 0 0 14px;
        font-size: 20px;
        font-weight: 600;
        color: #1e1b4b;
      }
      .knowledge-form-field {
        display: flex;
        flex-direction: column;
        gap: 6px;
        margin-bottom: 14px;
      }
      .knowledge-form-field label {
        font-size: 12px;
        font-weight: 600;
        color: #3f3f46;
      }
      .knowledge-form-field input,
      .knowledge-form-field select,
      .knowledge-form-field textarea {
        padding: 9px 12px;
        border-radius: 8px;
        border: 1px solid #ece9f5;
        font-family: inherit;
        font-size: 13px;
        color: #1e1b4b;
      }
      .knowledge-form-field textarea {
        font-family: var(--font-mono);
        min-height: 140px;
        resize: vertical;
      }
      .knowledge-modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
        margin-top: 8px;
      }
      .knowledge-modal-cancel {
        padding: 9px 14px;
        background: transparent;
        border: 1px solid #ece9f5;
        border-radius: 8px;
        font-weight: 600;
        font-size: 13px;
        cursor: pointer;
      }
      .knowledge-diff-pre {
        background: #fafaff;
        border: 1px solid #ece9f5;
        border-radius: 12px;
        padding: 18px;
        font-family: var(--font-mono);
        font-size: 12.5px;
        line-height: 1.55;
        white-space: pre-wrap;
        overflow-x: auto;
        color: #1e1b4b;
      }
      .knowledge-error-banner {
        background: rgba(185, 28, 28, 0.08);
        border: 1px solid rgba(185, 28, 28, 0.2);
        border-radius: 8px;
        padding: 10px 14px;
        color: #b91c1c;
        font-size: 13px;
        margin-bottom: 18px;
      }
    `}</style>
  );
}
