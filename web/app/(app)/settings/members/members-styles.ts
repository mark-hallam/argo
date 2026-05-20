/**
 * CSS-in-JS string for `/settings/members`. Lives in a sibling module
 * so the page + modal components stay well under the 500-LOC ceiling
 * once the cohort-scope surfaces (extra column, edit modal, picker)
 * join the route.
 */

export const MEMBERS_PAGE_STYLES = `
  .members-stage {
    padding: 48px 32px 96px;
    max-width: 1100px;
    margin: 0 auto;
  }
  .members-header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    margin-bottom: 36px;
    gap: 16px;
    flex-wrap: wrap;
  }
  .members-eyebrow {
    font-family: "JetBrains Mono", monospace;
    font-size: 11px;
    color: var(--muted-foreground, #71717a);
    letter-spacing: 0.16em;
    text-transform: uppercase;
    font-weight: 700;
    margin-bottom: 6px;
  }
  .members-title {
    font-family: "Inter", sans-serif;
    font-size: 30px;
    font-weight: 600;
    color: var(--foreground, #1e1b4b);
    margin: 0 0 4px;
    letter-spacing: -0.01em;
  }
  .members-sub {
    font-size: 14.5px;
    color: var(--muted-foreground, #3f3f46);
    margin: 0;
  }
  .members-invite-btn {
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
  .members-role-tiers {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-bottom: 32px;
  }
  @media (max-width: 800px) {
    .members-role-tiers {
      grid-template-columns: repeat(2, 1fr);
    }
  }
  .role-tier {
    border: 1px solid var(--border, #ece9f5);
    border-radius: 12px;
    padding: 16px;
    background: white;
  }
  .role-tier-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .role-tier-icon {
    width: 26px;
    height: 26px;
    border-radius: 7px;
    display: grid;
    place-items: center;
    color: white;
  }
  .role-tier-title {
    font-weight: 700;
    font-size: 14.5px;
    color: var(--foreground, #1e1b4b);
  }
  .role-tier-blurb {
    font-size: 12.5px;
    color: var(--muted-foreground, #3f3f46);
    line-height: 1.5;
    margin: 0 0 10px;
  }
  .role-tier-can {
    font-size: 12px;
    color: var(--muted-foreground, #71717a);
    line-height: 1.5;
    margin: 0;
    padding-left: 14px;
    list-style: square;
  }
  .members-list {
    background: white;
    border: 1px solid var(--border, #ece9f5);
    border-radius: 12px;
    overflow: hidden;
  }
  .members-list-head,
  .member-row {
    display: grid;
    grid-template-columns: 2.2fr 0.9fr 1.4fr 0.9fr 0.9fr 40px;
    gap: 12px;
    padding: 13px 18px;
    align-items: center;
  }
  .members-list-head {
    font-family: "JetBrains Mono", monospace;
    font-size: 10.5px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    font-weight: 700;
    color: var(--muted-foreground, #71717a);
    background: var(--background, #fafaff);
    border-bottom: 1px solid var(--border, #ece9f5);
  }
  .member-row {
    font-size: 14px;
    border-bottom: 1px solid var(--border, #ece9f5);
  }
  .member-row:last-child {
    border-bottom: none;
  }
  .member-name-block {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .member-avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: linear-gradient(135deg, #c4b5fd, #fbcfe8);
    color: #1e1b4b;
    display: grid;
    place-items: center;
    font-weight: 600;
    font-size: 13px;
  }
  .member-name {
    font-weight: 600;
    color: var(--foreground, #1e1b4b);
  }
  .member-email {
    font-size: 12.5px;
    color: var(--muted-foreground, #71717a);
  }
  .member-role-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border-radius: 999px;
    font-family: "JetBrains Mono", monospace;
    font-size: 10.5px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    font-weight: 700;
    color: white;
  }
  .member-status-pill {
    font-family: "JetBrains Mono", monospace;
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-weight: 700;
    padding: 3px 9px;
    border-radius: 999px;
  }
  .member-status-active {
    background: rgba(20, 184, 166, 0.15);
    color: #0f766e;
  }
  .member-status-invited {
    background: rgba(245, 158, 11, 0.15);
    color: #b45309;
  }
  .member-row-actions {
    background: transparent;
    border: none;
    color: var(--muted-foreground, #71717a);
    cursor: pointer;
    padding: 6px;
    border-radius: 6px;
    display: grid;
    place-items: center;
  }
  .member-row-actions:hover {
    background: rgba(0, 0, 0, 0.04);
    color: var(--foreground, #1e1b4b);
  }
  .member-scope-cell {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    position: relative;
  }
  .member-scope-chip {
    font-family: "Inter", sans-serif;
    font-size: 11.5px;
    font-weight: 600;
    color: #4c1d95;
    background: rgba(139, 92, 246, 0.1);
    border: 1px solid rgba(139, 92, 246, 0.18);
    border-radius: 6px;
    padding: 2px 8px;
    white-space: nowrap;
  }
  .member-scope-chip-all {
    background: rgba(20, 184, 166, 0.1);
    color: #0f766e;
    border-color: rgba(20, 184, 166, 0.22);
  }
  .member-scope-more {
    font-family: "JetBrains Mono", monospace;
    font-size: 10.5px;
    font-weight: 700;
    color: var(--muted-foreground, #71717a);
    background: transparent;
    border: 1px dashed var(--border, #ece9f5);
    border-radius: 6px;
    padding: 2px 8px;
    cursor: help;
  }
  .member-edit-btn {
    background: transparent;
    border: 1px solid var(--border, #ece9f5);
    border-radius: 8px;
    padding: 5px 12px;
    color: var(--muted-foreground, #3f3f46);
    cursor: pointer;
    font-size: 12.5px;
    font-weight: 600;
  }
  .member-edit-btn:hover {
    border-color: #8b5cf6;
    color: #6d28d9;
  }
  .invite-overlay {
    position: fixed;
    inset: 0;
    background: rgba(30, 27, 75, 0.5);
    backdrop-filter: blur(4px);
    display: grid;
    place-items: center;
    z-index: 100;
  }
  .invite-modal {
    background: white;
    border-radius: 16px;
    padding: 28px;
    width: 100%;
    max-width: 520px;
    box-shadow: 0 30px 80px -20px rgba(0, 0, 0, 0.4);
    max-height: 90vh;
    overflow-y: auto;
  }
  .invite-modal h2 {
    font-family: "Inter", sans-serif;
    font-size: 22px;
    font-weight: 600;
    margin: 0 0 6px;
    color: var(--foreground, #1e1b4b);
  }
  .invite-modal p {
    font-size: 14px;
    color: var(--muted-foreground, #3f3f46);
    margin: 0 0 22px;
  }
  .invite-label {
    font-family: "JetBrains Mono", monospace;
    font-size: 10.5px;
    color: var(--muted-foreground, #71717a);
    letter-spacing: 0.16em;
    text-transform: uppercase;
    font-weight: 700;
    margin-bottom: 8px;
    display: block;
  }
  .invite-input {
    width: 100%;
    padding: 12px 16px;
    border-radius: 10px;
    border: 1px solid var(--border, #ece9f5);
    font-family: "Inter", sans-serif;
    font-size: 15px;
    margin-bottom: 16px;
  }
  .invite-input:focus {
    outline: none;
    border-color: #8b5cf6;
    box-shadow: 0 0 0 4px rgba(139, 92, 246, 0.1);
  }
  .invite-role-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
    margin-bottom: 22px;
  }
  .invite-role-pill {
    padding: 9px;
    border-radius: 9px;
    border: 1px solid var(--border, #ece9f5);
    background: white;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    color: var(--muted-foreground, #71717a);
  }
  .invite-role-pill.selected {
    border-color: #8b5cf6;
    background: rgba(139, 92, 246, 0.08);
    color: #6d28d9;
  }
  .invite-actions {
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }
  .invite-cancel {
    padding: 11px 18px;
    border-radius: 10px;
    background: transparent;
    color: var(--foreground, #1e1b4b);
    border: 1px solid var(--border, #ece9f5);
    font-weight: 600;
    font-size: 14px;
    cursor: pointer;
  }
  .invite-send {
    padding: 11px 22px;
    border-radius: 10px;
    background: linear-gradient(135deg, #8b5cf6, #ec4899);
    color: white;
    border: none;
    font-weight: 700;
    font-size: 14px;
    cursor: pointer;
  }
  .invite-send:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .scope-toggle-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-bottom: 12px;
  }
  .scope-toggle-pill {
    padding: 10px 12px;
    border-radius: 10px;
    border: 1px solid var(--border, #ece9f5);
    background: white;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    color: var(--muted-foreground, #3f3f46);
    text-align: left;
    line-height: 1.3;
  }
  .scope-toggle-pill.selected {
    border-color: #8b5cf6;
    background: rgba(139, 92, 246, 0.08);
    color: #6d28d9;
  }
  .scope-toggle-pill:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .scope-toggle-hint {
    font-size: 11px;
    font-weight: 500;
    color: var(--muted-foreground, #71717a);
    margin-top: 4px;
    display: block;
    letter-spacing: 0;
    text-transform: none;
    font-family: "Inter", sans-serif;
  }
  .scope-cohort-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 8px;
    padding: 12px;
    border-radius: 10px;
    background: var(--background, #fafaff);
    border: 1px solid var(--border, #ece9f5);
    max-height: 220px;
    overflow-y: auto;
  }
  .scope-cohort-chip {
    padding: 6px 12px;
    border-radius: 999px;
    border: 1px solid var(--border, #ece9f5);
    background: white;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--muted-foreground, #71717a);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .scope-cohort-chip.selected {
    border-color: #8b5cf6;
    background: rgba(139, 92, 246, 0.1);
    color: #4c1d95;
  }
  .scope-cohort-empty {
    font-size: 12.5px;
    color: var(--muted-foreground, #71717a);
    padding: 4px;
  }
  .scope-helper {
    font-size: 12.5px;
    color: var(--muted-foreground, #71717a);
    margin: 0 0 18px;
    line-height: 1.45;
  }
  .scope-override-note {
    font-size: 12.5px;
    color: #b45309;
    background: rgba(245, 158, 11, 0.08);
    border: 1px solid rgba(245, 158, 11, 0.22);
    border-radius: 8px;
    padding: 9px 12px;
    margin-bottom: 18px;
    line-height: 1.45;
  }
`;
