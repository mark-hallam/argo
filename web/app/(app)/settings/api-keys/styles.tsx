/** Inline page styles for `/settings/api-keys`. Kept in a sibling file so
 * the page component stays under the in-page LOC budget; the styles are
 * scoped via class prefixes (`ak-*`) and don't bleed into other routes. */

export function ApiKeysStyles() {
  return (
    <style jsx global>{`
      .api-keys-stage {
        padding: 48px 32px 96px;
        max-width: 1180px;
        margin: 0 auto;
      }
      .ak-header {
        display: flex;
        align-items: flex-end;
        justify-content: space-between;
        margin-bottom: 28px;
        gap: 16px;
        flex-wrap: wrap;
      }
      .ak-eyebrow {
        font-family: "JetBrains Mono", monospace;
        font-size: 11px;
        color: var(--muted-foreground, #71717a);
        letter-spacing: 0.16em;
        text-transform: uppercase;
        font-weight: 700;
        margin-bottom: 6px;
      }
      .ak-title {
        font-family: "Inter", sans-serif;
        font-size: 30px;
        font-weight: 600;
        color: var(--foreground, #1e1b4b);
        margin: 0 0 4px;
        letter-spacing: -0.01em;
      }
      .ak-sub {
        font-size: 14.5px;
        color: var(--muted-foreground, #3f3f46);
        margin: 0;
      }
      .ak-create-btn {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 11px 18px;
        border-radius: 10px;
        background: linear-gradient(135deg, #8b5cf6, #ec4899);
        color: white;
        font-weight: 600;
        font-size: 14px;
        border: none;
        cursor: pointer;
        box-shadow: 0 6px 16px -4px rgba(139, 92, 246, 0.4);
      }
      .ak-create-btn:hover {
        filter: brightness(1.05);
      }
      .ak-list {
        background: white;
        border: 1px solid var(--border, #ece9f5);
        border-radius: 12px;
        overflow: hidden;
      }
      .ak-list-head,
      .ak-row {
        display: grid;
        grid-template-columns: 1.3fr 1.3fr 1.7fr 0.9fr 0.9fr 1.1fr 110px;
        gap: 12px;
        padding: 13px 18px;
        align-items: center;
      }
      .ak-list-head {
        font-family: "JetBrains Mono", monospace;
        font-size: 10.5px;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        font-weight: 700;
        color: var(--muted-foreground, #71717a);
        background: var(--background, #fafaff);
        border-bottom: 1px solid var(--border, #ece9f5);
      }
      .ak-row {
        font-size: 13.5px;
        border-bottom: 1px solid var(--border, #ece9f5);
      }
      .ak-row:last-child {
        border-bottom: none;
      }
      .ak-row-revoked {
        opacity: 0.55;
      }
      .ak-row-name {
        display: flex;
        align-items: center;
        gap: 8px;
        font-weight: 600;
        color: var(--foreground, #1e1b4b);
      }
      .ak-row-name-text {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
      .ak-row-prefix {
        font-family: "JetBrains Mono", monospace;
        font-size: 12px;
        color: var(--muted-foreground, #3f3f46);
        background: var(--background, #fafaff);
        padding: 4px 8px;
        border-radius: 6px;
        border: 1px solid var(--border, #ece9f5);
        width: fit-content;
      }
      .ak-row-scopes {
        display: flex;
        gap: 4px;
        flex-wrap: wrap;
      }
      .ak-row-time {
        font-size: 12.5px;
        color: var(--muted-foreground, #71717a);
      }
      .ak-row-expires-warn {
        color: #b45309;
        font-weight: 600;
      }
      .ak-row-expires-stale {
        color: #b91c1c;
        font-weight: 600;
      }
      .ak-pill {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        font-family: "JetBrains Mono", monospace;
        font-size: 10px;
        letter-spacing: 0.06em;
        padding: 3px 7px;
        border-radius: 999px;
        font-weight: 700;
        border: 1px solid var(--border, #ece9f5);
        background: white;
        color: var(--muted-foreground, #3f3f46);
        white-space: nowrap;
      }
      .ak-pill-scope {
        background: rgba(139, 92, 246, 0.08);
        color: #6d28d9;
        border-color: rgba(139, 92, 246, 0.2);
      }
      .ak-pill-cohort {
        background: rgba(8, 145, 178, 0.08);
        color: #0e7490;
        border-color: rgba(8, 145, 178, 0.2);
      }
      .ak-pill-revoked {
        background: rgba(220, 38, 38, 0.08);
        color: #b91c1c;
        border-color: rgba(220, 38, 38, 0.25);
      }
      .ak-row-actions {
        display: flex;
        justify-content: flex-end;
      }
      .ak-revoke-btn {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        background: transparent;
        border: 1px solid var(--border, #ece9f5);
        color: #b91c1c;
        font-size: 12px;
        font-weight: 600;
        padding: 6px 10px;
        border-radius: 8px;
        cursor: pointer;
      }
      .ak-revoke-btn:hover {
        background: rgba(220, 38, 38, 0.08);
        border-color: rgba(220, 38, 38, 0.35);
      }
      .ak-empty {
        background: white;
        border: 1px solid var(--border, #ece9f5);
        border-radius: 16px;
        padding: 56px 28px;
        text-align: center;
        max-width: 540px;
        margin: 32px auto;
      }
      .ak-empty-icon {
        width: 56px;
        height: 56px;
        border-radius: 14px;
        background: rgba(139, 92, 246, 0.08);
        color: #8b5cf6;
        display: grid;
        place-items: center;
        margin: 0 auto 18px;
      }
      .ak-empty-title {
        font-family: "Inter", sans-serif;
        font-size: 20px;
        font-weight: 600;
        margin: 0 0 6px;
        color: var(--foreground, #1e1b4b);
      }
      .ak-empty-blurb {
        font-size: 14px;
        color: var(--muted-foreground, #3f3f46);
        max-width: 380px;
        margin: 0 auto 20px;
        line-height: 1.55;
      }
      .ak-overlay {
        position: fixed;
        inset: 0;
        background: rgba(30, 27, 75, 0.5);
        backdrop-filter: blur(4px);
        display: grid;
        place-items: center;
        z-index: 100;
        padding: 24px;
      }
      .ak-modal {
        background: white;
        border-radius: 16px;
        padding: 28px;
        width: 100%;
        max-width: 560px;
        max-height: calc(100vh - 48px);
        overflow: auto;
        box-shadow: 0 30px 80px -20px rgba(0, 0, 0, 0.4);
        position: relative;
      }
      .ak-modal-small {
        max-width: 440px;
      }
      .ak-modal-close {
        position: absolute;
        top: 14px;
        right: 14px;
        background: transparent;
        border: none;
        color: var(--muted-foreground, #71717a);
        cursor: pointer;
        padding: 6px;
        border-radius: 6px;
      }
      .ak-modal-close:hover {
        background: rgba(0, 0, 0, 0.04);
      }
      .ak-modal h2 {
        font-family: "Inter", sans-serif;
        font-size: 22px;
        font-weight: 600;
        margin: 0 0 6px;
        color: var(--foreground, #1e1b4b);
      }
      .ak-modal-blurb {
        font-size: 13.5px;
        color: var(--muted-foreground, #3f3f46);
        margin: 0 0 18px;
        line-height: 1.5;
      }
      .ak-modal-meta {
        font-size: 12.5px;
        color: var(--muted-foreground, #71717a);
        margin: 8px 0 18px;
      }
      .ak-label {
        font-family: "JetBrains Mono", monospace;
        font-size: 10.5px;
        color: var(--muted-foreground, #71717a);
        letter-spacing: 0.16em;
        text-transform: uppercase;
        font-weight: 700;
        margin: 14px 0 8px;
        display: block;
      }
      .ak-input {
        width: 100%;
        padding: 11px 14px;
        border-radius: 10px;
        border: 1px solid var(--border, #ece9f5);
        font-family: "Inter", sans-serif;
        font-size: 14.5px;
        background: white;
        color: var(--foreground, #1e1b4b);
      }
      .ak-input:focus {
        outline: none;
        border-color: #8b5cf6;
        box-shadow: 0 0 0 4px rgba(139, 92, 246, 0.1);
      }
      .ak-scope-grid {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        gap: 14px;
        margin-bottom: 6px;
      }
      @media (max-width: 600px) {
        .ak-scope-grid {
          grid-template-columns: 1fr;
        }
      }
      .ak-scope-group {
        border: 1px solid var(--border, #ece9f5);
        border-radius: 10px;
        padding: 10px 12px;
        background: var(--background, #fafaff);
      }
      .ak-scope-group-title {
        font-family: "JetBrains Mono", monospace;
        font-size: 10px;
        letter-spacing: 0.14em;
        text-transform: uppercase;
        color: var(--muted-foreground, #71717a);
        margin-bottom: 6px;
        font-weight: 700;
      }
      .ak-scope-check {
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 13px;
        padding: 4px 0;
        cursor: pointer;
        color: var(--foreground, #1e1b4b);
      }
      .ak-admin-pill {
        margin-left: 6px;
        font-family: "JetBrains Mono", monospace;
        font-size: 9.5px;
        letter-spacing: 0.06em;
        background: rgba(236, 72, 153, 0.12);
        color: #be185d;
        padding: 1px 5px;
        border-radius: 5px;
        font-weight: 700;
      }
      .ak-all-cohorts {
        margin-bottom: 8px;
      }
      .ak-cohort-grid {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        gap: 6px;
        padding: 10px 12px;
        border: 1px solid var(--border, #ece9f5);
        border-radius: 10px;
        background: var(--background, #fafaff);
      }
      .ak-cohort-check {
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 12.5px;
        cursor: pointer;
        color: var(--foreground, #1e1b4b);
      }
      .ak-cohort-vertical {
        font-weight: 600;
        margin-right: 6px;
      }
      .ak-cohort-band {
        font-family: "JetBrains Mono", monospace;
        font-size: 10px;
        letter-spacing: 0.06em;
        color: var(--muted-foreground, #71717a);
      }
      .ak-expiry-row {
        display: flex;
        gap: 6px;
        flex-wrap: wrap;
        margin-bottom: 12px;
      }
      .ak-expiry-pill {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 8px 14px;
        border-radius: 9px;
        border: 1px solid var(--border, #ece9f5);
        background: white;
        font-size: 13px;
        font-weight: 600;
        cursor: pointer;
        color: var(--muted-foreground, #3f3f46);
      }
      .ak-expiry-pill input {
        display: none;
      }
      .ak-expiry-pill.selected {
        border-color: #8b5cf6;
        background: rgba(139, 92, 246, 0.08);
        color: #6d28d9;
      }
      .ak-warning {
        display: flex;
        align-items: center;
        gap: 10px;
        background: rgba(245, 158, 11, 0.1);
        border: 1px solid rgba(245, 158, 11, 0.4);
        color: #92400e;
        padding: 11px 14px;
        border-radius: 10px;
        font-size: 13px;
        font-weight: 600;
        margin-bottom: 14px;
      }
      .ak-token-box {
        display: flex;
        align-items: center;
        gap: 10px;
        background: var(--background, #fafaff);
        border: 1px solid var(--border, #ece9f5);
        border-radius: 10px;
        padding: 12px 14px;
        margin-bottom: 8px;
        font-family: "JetBrains Mono", monospace;
        font-size: 13px;
        color: var(--foreground, #1e1b4b);
        overflow: hidden;
      }
      .ak-token-box code {
        flex: 1;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        user-select: all;
      }
      .ak-copy-btn {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        border: 1px solid var(--border, #ece9f5);
        background: white;
        color: var(--foreground, #1e1b4b);
        padding: 6px 12px;
        border-radius: 8px;
        cursor: pointer;
        font-size: 12.5px;
        font-weight: 600;
        font-family: "Inter", sans-serif;
        flex-shrink: 0;
      }
      .ak-copy-btn:hover {
        background: rgba(139, 92, 246, 0.06);
        border-color: rgba(139, 92, 246, 0.4);
      }
      .ak-confirm-inline {
        background: rgba(220, 38, 38, 0.06);
        border: 1px solid rgba(220, 38, 38, 0.25);
        border-radius: 10px;
        padding: 12px 14px;
        margin-top: 14px;
        color: #7f1d1d;
        font-size: 13px;
        display: flex;
        flex-direction: column;
        gap: 10px;
      }
      .ak-confirm-inline-actions {
        display: flex;
        gap: 8px;
        justify-content: flex-end;
      }
      .ak-actions {
        display: flex;
        gap: 10px;
        justify-content: flex-end;
        margin-top: 18px;
      }
      .ak-cancel {
        padding: 10px 16px;
        border-radius: 10px;
        background: transparent;
        color: var(--foreground, #1e1b4b);
        border: 1px solid var(--border, #ece9f5);
        font-weight: 600;
        font-size: 13.5px;
        cursor: pointer;
      }
      .ak-cancel:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }
      .ak-submit {
        padding: 10px 18px;
        border-radius: 10px;
        background: linear-gradient(135deg, #8b5cf6, #ec4899);
        color: white;
        border: none;
        font-weight: 700;
        font-size: 13.5px;
        cursor: pointer;
      }
      .ak-submit:disabled {
        opacity: 0.55;
        cursor: not-allowed;
        filter: grayscale(0.2);
      }
      .ak-submit-danger {
        background: linear-gradient(135deg, #ef4444, #b91c1c);
      }
      .ak-error {
        margin-top: 12px;
        font-size: 13px;
        color: #b91c1c;
      }
    `}</style>
  );
}
