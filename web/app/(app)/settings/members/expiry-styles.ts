/**
 * CSS-in-JS string for the time-bound-access surfaces in
 * `/settings/members`. Lives next to `members-styles.ts` so the
 * shared sheet stays under the 500-LOC ceiling. Page composes the
 * two strings into a single `<style jsx global>` block.
 */

export const MEMBERS_EXPIRY_STYLES = `
  .member-row[data-expired="true"] {
    opacity: 0.55;
    background: rgba(0, 0, 0, 0.015);
  }
  .member-expiry-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 3px;
    font-family: "JetBrains Mono", monospace;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 999px;
  }
  .member-expiry-pill[data-tone="green"] {
    background: rgba(20, 184, 166, 0.15);
    color: #0f766e;
  }
  .member-expiry-pill[data-tone="amber"] {
    background: rgba(245, 158, 11, 0.18);
    color: #b45309;
  }
  .member-expiry-pill[data-tone="red"] {
    background: rgba(239, 68, 68, 0.18);
    color: #b91c1c;
  }
  .invite-expiry-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 6px;
    margin-bottom: 8px;
  }
  @media (max-width: 520px) {
    .invite-expiry-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }
  .invite-expiry-pill {
    padding: 9px 4px;
    border-radius: 9px;
    border: 1px solid var(--border, #ece9f5);
    background: white;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    color: var(--muted-foreground, #52525b);
    text-align: center;
  }
  .invite-expiry-pill.selected {
    border-color: #8b5cf6;
    background: rgba(139, 92, 246, 0.08);
    color: #6d28d9;
  }
  .invite-expiry-helper {
    font-size: 12px;
    color: var(--muted-foreground, #52525b);
    line-height: 1.5;
    margin: 0 0 18px;
  }
  .invite-expiry-date {
    width: 100%;
    padding: 9px 12px;
    border-radius: 8px;
    border: 1px solid var(--border, #ece9f5);
    font-family: "Inter", sans-serif;
    font-size: 14px;
    margin-bottom: 10px;
  }
`;
