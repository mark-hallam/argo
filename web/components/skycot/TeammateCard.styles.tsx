"use client";

/**
 * Extracted as global styled-jsx so the parent TeammateCard keeps simple
 * class names while preserving the locked C2 holographic showcase styling.
 */
export function TeammateCardStyles() {
  return (
    <style jsx global>{`
      .holo-card {
        position: relative;
        border-radius: 24px;
        overflow: hidden;
        background: var(--card, white);
        box-shadow:
          0 1px 2px rgba(0, 0, 0, 0.06),
          0 32px 60px -16px rgba(139, 92, 246, 0.3),
          inset 0 0 0 2px rgba(139, 92, 246, 0.12);
        transition:
          transform 0.35s cubic-bezier(0.2, 0, 0, 1),
          box-shadow 0.35s cubic-bezier(0.2, 0, 0, 1);
        cursor: pointer;
        outline: none;
        display: flex;
        flex-direction: column;
      }
      .holo-card:focus-visible {
        box-shadow:
          0 1px 2px rgba(0, 0, 0, 0.06),
          0 32px 60px -16px rgba(139, 92, 246, 0.3),
          inset 0 0 0 2px rgba(139, 92, 246, 0.12),
          0 0 0 3px var(--ring);
      }
      .holo-card:hover {
        transform: translateY(-8px) rotate(-1deg) scale(1.015);
        box-shadow:
          0 1px 2px rgba(0, 0, 0, 0.06),
          0 48px 80px -16px rgba(139, 92, 246, 0.45),
          inset 0 0 0 2px rgba(139, 92, 246, 0.25);
      }
      .holo-banner {
        position: relative;
        background: linear-gradient(135deg, var(--primary) 0%, var(--gradient-end) 100%);
        color: white;
        padding: 24px 22px 72px;
        overflow: hidden;
      }
      .holo-banner::before {
        content: "";
        position: absolute;
        inset: 0;
        background:
          radial-gradient(circle at 30% 20%, rgba(255, 255, 255, 0.5), transparent 40%),
          radial-gradient(circle at 80% 80%, rgba(255, 255, 255, 0.25), transparent 50%);
        pointer-events: none;
      }
      .holo-shine {
        position: absolute;
        inset: 0;
        background: linear-gradient(
          115deg,
          transparent 35%,
          rgba(255, 255, 255, 0.6) 50%,
          transparent 65%
        );
        background-size: 200% 200%;
        background-position: 200% 50%;
        transition: background-position 1s cubic-bezier(0.2, 0, 0, 1);
        pointer-events: none;
      }
      .holo-card:hover .holo-shine {
        background-position: -50% 50%;
      }
      @media (prefers-reduced-motion: reduce) {
        .holo-card {
          transition: none;
        }
        .holo-card:hover {
          transform: none;
        }
        .holo-shine {
          display: none;
        }
      }
      .holo-banner-row {
        position: relative;
        z-index: 1;
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 12px;
      }
      .holo-banner-tag {
        font-size: 10.5px;
        font-weight: 800;
        text-transform: uppercase;
        letter-spacing: 0.16em;
        padding: 5px 12px;
        border-radius: 999px;
        background: rgba(255, 255, 255, 0.2);
        backdrop-filter: blur(10px);
        -webkit-backdrop-filter: blur(10px);
        white-space: nowrap;
      }
      .holo-banner-level {
        text-align: right;
        line-height: 1;
        min-width: 0;
      }
      .holo-banner-level-lbl {
        font-size: 9.5px;
        font-weight: 700;
        letter-spacing: 0.18em;
        opacity: 0.85;
      }
      .holo-banner-level-n {
        font-size: 56px;
        font-weight: 900;
        line-height: 0.9;
        letter-spacing: -0.04em;
        margin-top: 2px;
        text-shadow: 0 4px 16px rgba(0, 0, 0, 0.18);
      }
      .holo-menu-slot {
        position: absolute;
        top: 14px;
        right: 14px;
        z-index: 3;
        display: inline-flex;
      }
      .holo-portrait {
        position: relative;
        z-index: 2;
        margin: -56px 22px 0;
        display: flex;
        align-items: flex-end;
        height: 100px;
      }
      .holo-avatar {
        width: 92px;
        height: 92px;
        border-radius: 26px;
        background: var(--card, white);
        padding: 5px;
        box-shadow: 0 16px 40px -8px rgba(0, 0, 0, 0.2);
      }
      .holo-avatar-inner {
        width: 100%;
        height: 100%;
        border-radius: 21px;
        background: linear-gradient(135deg, var(--primary) 0%, var(--gradient-end) 100%);
        display: grid;
        place-items: center;
        color: white;
        font-size: 28px;
        font-weight: 800;
        letter-spacing: 0.02em;
      }
      .holo-name-block {
        padding: 14px 24px 0;
      }
      .holo-name {
        margin: 0;
        font-size: 26px;
        font-weight: 800;
        letter-spacing: -0.025em;
        color: var(--foreground);
        line-height: 1.1;
      }
      .holo-role {
        margin-top: 4px;
        color: var(--muted-foreground);
        font-size: 13px;
        font-weight: 600;
      }
      .holo-status-row {
        margin-top: 12px;
      }
      .holo-traits {
        padding: 16px 24px 0;
      }
      .holo-traits-lbl {
        font-size: 9.5px;
        text-transform: uppercase;
        letter-spacing: 0.18em;
        color: var(--muted-foreground);
        font-weight: 800;
        margin-bottom: 8px;
      }
      .holo-trait-chips {
        display: flex;
        flex-wrap: wrap;
        gap: 6px;
      }
      .holo-trait-chip {
        font-size: 11px;
        font-weight: 600;
        padding: 4px 10px;
        border-radius: 999px;
        background: rgba(139, 92, 246, 0.08);
        color: #6d28d9;
        border: 1px solid rgba(139, 92, 246, 0.15);
      }
      .holo-body {
        padding: 18px 24px 0;
        display: flex;
        flex-direction: column;
        gap: 10px;
        flex: 1;
      }
      .holo-block-lbl {
        font-size: 9.5px;
        text-transform: uppercase;
        letter-spacing: 0.18em;
        color: var(--muted-foreground);
        font-weight: 800;
        margin-bottom: 4px;
      }
      .holo-bullets {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 4px;
        color: var(--foreground);
        font-size: 13px;
        line-height: 1.45;
      }
      .holo-bullet {
        display: flex;
        gap: 8px;
      }
      .holo-bullet::before {
        content: "•";
        color: var(--muted-foreground);
      }
      .holo-tomorrow {
        font-size: 12px;
        color: var(--muted-foreground);
      }
      .holo-tomorrow-lbl {
        font-weight: 700;
        color: var(--foreground);
      }
      .holo-foot {
        margin-top: auto;
        padding: 18px 24px 18px;
      }
    `}</style>
  );
}
