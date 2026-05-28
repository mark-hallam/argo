"use client";

/**
 * Global F1-editorial-center form/card class set, scoped under
 * `.wiz-editorial` (the `WizardEditorialShell` root). Companion to the
 * shell — the shell owns the brand chrome (Audiowide hero, gradient,
 * trust footer); this module styles the inside of `.wiz-body` so each
 * onboarding step renders the locked F1 mockup register (mono-uppercase
 * field labels, large inputs with subtle purple focus ring, selectable
 * cards with gradient check pip, mono skill pills, gradient CTA pill).
 *
 * Why a separate styles component instead of folding into the shell:
 * `/hire` also uses the shell but renders its own `Step1Land`...`Step5Connect`
 * components which have their own Tailwind-on-CSS-tokens look. Keeping
 * these classes additive — and scoped to `.wiz-editorial .ob-*` — lets
 * the onboarding flow opt into F1 form chrome without affecting the
 * existing `/hire` surface.
 *
 * The CSS variable namespace (--ob-purple, --ob-line, etc.) re-declares
 * the brand palette under the same `.wiz-editorial` so we don't depend
 * on accidental token leakage from the shell.
 */
export function OnboardingFormStyles() {
  return (
    <style jsx global>{`
      .wiz-editorial {
        --ob-purple: #8b5cf6;
        --ob-purple-deep: #6d28d9;
        --ob-pink: #ec4899;
        --ob-coral: #ff6b6b;
        --ob-yellow: #fcd34d;
        --ob-teal: #14b8a6;
        --ob-cyan: #0891b2;
        --ob-ink: #1e1b4b;
        --ob-ink-2: #3f3f46;
        --ob-ink-3: #52525b;
        --ob-ink-4: #a1a1aa;
        --ob-bg: #fafaff;
        --ob-paper: #fafaf9;
        --ob-paper-2: #f4f4f4;
        --ob-line: #ece9f5;
        --ob-line-2: #d1d1d1;
        --ob-gradient: linear-gradient(135deg, var(--ob-purple), var(--ob-pink));
        --ob-gradient-3: linear-gradient(
          135deg,
          var(--ob-purple),
          var(--ob-pink),
          var(--ob-coral)
        );
      }
      .wiz-editorial .ob-stack {
        display: flex;
        flex-direction: column;
        gap: 20px;
      }
      .wiz-editorial .ob-field {
        display: block;
        text-align: left;
      }
      .wiz-editorial .ob-field-label {
        display: block;
        font-family: var(--font-mono);
        font-size: 10.5px;
        color: var(--ob-ink-3);
        text-transform: uppercase;
        letter-spacing: 0.16em;
        font-weight: 700;
        margin-bottom: 8px;
      }
      .wiz-editorial .ob-field-input,
      .wiz-editorial .ob-field-textarea {
        width: 100%;
        padding: 14px 18px;
        border-radius: 10px;
        border: 1px solid var(--ob-line);
        background: var(--ob-bg);
        font-family: "Inter", sans-serif;
        font-size: 16px;
        font-weight: 500;
        color: var(--ob-ink);
        letter-spacing: -0.005em;
        outline: none;
        transition:
          border-color 0.15s,
          box-shadow 0.15s;
      }
      .wiz-editorial .ob-field-textarea {
        min-height: 144px;
        resize: vertical;
        font-weight: 400;
        line-height: 1.5;
      }
      .wiz-editorial .ob-field-input::placeholder,
      .wiz-editorial .ob-field-textarea::placeholder {
        color: var(--ob-ink-4);
        font-weight: 400;
      }
      .wiz-editorial .ob-field-input:focus,
      .wiz-editorial .ob-field-textarea:focus {
        border-color: var(--ob-purple);
        box-shadow: 0 0 0 4px rgba(139, 92, 246, 0.1);
      }
      .wiz-editorial .ob-field-input.is-mono {
        font-family: var(--font-mono);
        font-size: 14px;
        letter-spacing: 0.02em;
      }
      .wiz-editorial .ob-field-hint {
        margin-top: 8px;
        font-size: 12.5px;
        color: var(--ob-ink-3);
      }
      .wiz-editorial .ob-field-hint strong {
        color: var(--ob-ink);
        font-weight: 600;
      }
      .wiz-editorial .ob-field-hint a {
        color: var(--ob-purple-deep);
        text-decoration: none;
        border-bottom: 1px solid var(--ob-line-2);
        padding-bottom: 1px;
        font-weight: 600;
      }
      .wiz-editorial .ob-field-hint a:hover {
        color: var(--ob-purple);
        border-color: var(--ob-purple);
      }
      .wiz-editorial .ob-form-error {
        display: flex;
        align-items: flex-start;
        gap: 8px;
        font-size: 13px;
        color: #b91c1c;
        margin: 4px 0;
      }
      .wiz-editorial .ob-form-success {
        display: flex;
        align-items: flex-start;
        gap: 8px;
        font-size: 13px;
        color: #047857;
        margin: 4px 0;
      }
      .wiz-editorial .ob-form-note {
        padding: 12px 14px;
        border-radius: 10px;
        background: rgba(252, 211, 77, 0.12);
        border: 1px solid rgba(252, 211, 77, 0.4);
        color: #854d0e;
        font-size: 12.5px;
        line-height: 1.5;
        text-align: left;
      }
      .wiz-editorial .ob-cta-row {
        margin-top: 12px;
        display: flex;
        gap: 12px;
        justify-content: center;
        align-items: center;
        flex-wrap: wrap;
      }
      .wiz-editorial .ob-cta-row.is-split {
        justify-content: space-between;
      }
      .wiz-editorial .ob-cta {
        display: inline-flex;
        align-items: center;
        gap: 10px;
        padding: 15px 30px;
        border-radius: 12px;
        font-family: "Inter", sans-serif;
        font-size: 15px;
        font-weight: 700;
        letter-spacing: -0.005em;
        color: white;
        background: var(--ob-gradient);
        border: none;
        cursor: pointer;
        box-shadow:
          0 8px 24px -8px rgba(139, 92, 246, 0.45),
          inset 0 1px 0 rgba(255, 255, 255, 0.25);
        transition:
          transform 0.15s,
          box-shadow 0.15s,
          opacity 0.15s;
      }
      .wiz-editorial .ob-cta:hover:not(:disabled) {
        transform: translateY(-2px);
        box-shadow:
          0 12px 32px -8px rgba(139, 92, 246, 0.6),
          inset 0 1px 0 rgba(255, 255, 255, 0.3);
      }
      .wiz-editorial .ob-cta:disabled,
      .wiz-editorial .ob-cta[aria-busy="true"] {
        opacity: 0.55;
        cursor: not-allowed;
        box-shadow: none;
      }
      .wiz-editorial .ob-cta-back,
      .wiz-editorial .ob-cta-ghost {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        padding: 15px 22px;
        border-radius: 12px;
        font-family: "Inter", sans-serif;
        font-size: 14.5px;
        font-weight: 600;
        background: transparent;
        color: var(--ob-ink-2);
        border: 1px solid var(--ob-line);
        cursor: pointer;
        box-shadow: none;
        transition:
          border-color 0.15s,
          background 0.15s,
          transform 0.15s,
          opacity 0.15s;
      }
      .wiz-editorial .ob-cta-back:hover:not(:disabled),
      .wiz-editorial .ob-cta-ghost:hover:not(:disabled) {
        border-color: var(--ob-line-2);
        background: rgba(255, 255, 255, 0.55);
        transform: translateY(-1px);
      }
      .wiz-editorial .ob-cta-back:disabled,
      .wiz-editorial .ob-cta-ghost:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }
      .wiz-editorial .ob-cta-kbd {
        font-family: var(--font-mono);
        font-size: 11px;
        padding: 2px 7px;
        border-radius: 5px;
        background: rgba(255, 255, 255, 0.22);
        letter-spacing: 0.04em;
      }
      .wiz-editorial .ob-form-card {
        margin: 0 auto;
        max-width: 520px;
        padding: 28px 28px 24px;
        border-radius: 18px;
        border: 1px solid var(--ob-line);
        background: rgba(255, 255, 255, 0.85);
        box-shadow: 0 12px 36px -16px rgba(30, 27, 75, 0.12);
        display: flex;
        flex-direction: column;
        gap: 18px;
        text-align: left;
      }
      .wiz-editorial .ob-pillars {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 14px;
        max-width: 760px;
        margin: 0 auto;
      }
      .wiz-editorial .ob-pillar {
        text-align: left;
        padding: 18px 18px 16px;
        border-radius: 14px;
        border: 1px solid var(--ob-line);
        background: rgba(255, 255, 255, 0.78);
        box-shadow: 0 8px 22px -16px rgba(30, 27, 75, 0.18);
      }
      .wiz-editorial .ob-pillar-label {
        font-family: var(--font-mono);
        font-size: 10.5px;
        color: var(--ob-ink-3);
        text-transform: uppercase;
        letter-spacing: 0.16em;
        font-weight: 700;
        margin-bottom: 8px;
      }
      .wiz-editorial .ob-pillar-body {
        font-size: 13.5px;
        line-height: 1.5;
        color: var(--ob-ink-2);
      }
      .wiz-editorial .ob-roles {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 16px;
      }
      @media (min-width: 920px) {
        .wiz-editorial .ob-roles {
          grid-template-columns: repeat(3, minmax(0, 1fr));
        }
      }
      .wiz-editorial .ob-role-card {
        position: relative;
        display: flex;
        flex-direction: column;
        gap: 10px;
        padding: 22px 22px 20px;
        border-radius: 16px;
        border: 1px solid var(--ob-line);
        background: rgba(255, 255, 255, 0.85);
        text-align: left;
        font-family: "Inter", sans-serif;
        color: var(--ob-ink);
        cursor: pointer;
        transition:
          transform 0.15s,
          border-color 0.15s,
          box-shadow 0.15s;
      }
      .wiz-editorial .ob-role-card:hover:not(:disabled) {
        transform: translateY(-2px);
        border-color: var(--ob-line-2);
        box-shadow: 0 8px 22px -10px rgba(30, 27, 75, 0.14);
      }
      .wiz-editorial .ob-role-card.is-selected {
        border-color: var(--ob-purple);
        box-shadow:
          0 0 0 3px rgba(139, 92, 246, 0.12),
          0 12px 30px -10px rgba(139, 92, 246, 0.28);
      }
      .wiz-editorial .ob-role-card:focus-visible {
        outline: none;
        box-shadow:
          0 0 0 3px rgba(139, 92, 246, 0.25),
          0 12px 30px -10px rgba(139, 92, 246, 0.28);
      }
      .wiz-editorial .ob-role-card:disabled {
        cursor: wait;
        opacity: 0.75;
      }
      .wiz-editorial .ob-role-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
      }
      .wiz-editorial .ob-role-name {
        font-size: 17px;
        font-weight: 700;
        letter-spacing: -0.015em;
        color: var(--ob-ink);
      }
      .wiz-editorial .ob-role-check {
        width: 22px;
        height: 22px;
        border-radius: 50%;
        border: 1.5px solid var(--ob-line-2);
        background: var(--ob-bg);
        flex-shrink: 0;
        display: grid;
        place-items: center;
        font-size: 11px;
        font-weight: 700;
        color: transparent;
      }
      .wiz-editorial .ob-role-card.is-selected .ob-role-check {
        background: var(--ob-gradient);
        border: none;
        color: white;
      }
      .wiz-editorial .ob-role-desc {
        font-size: 13.5px;
        color: var(--ob-ink-2);
        line-height: 1.55;
      }
      .wiz-editorial .ob-skill-chips {
        display: flex;
        flex-wrap: wrap;
        gap: 5px;
      }
      .wiz-editorial .ob-skill-chip {
        padding: 3px 8px;
        border-radius: 6px;
        background: var(--ob-paper-2);
        color: var(--ob-ink-2);
        font-family: var(--font-mono);
        font-size: 10.5px;
        font-weight: 600;
        letter-spacing: 0.04em;
      }
      .wiz-editorial .ob-role-meta {
        margin-top: auto;
        padding-top: 10px;
        border-top: 1px solid var(--ob-line);
        font-family: var(--font-mono);
        font-size: 11px;
        color: var(--ob-ink-3);
        letter-spacing: 0.04em;
        text-transform: uppercase;
        font-weight: 600;
      }
      .wiz-editorial .ob-role-meta strong {
        color: var(--ob-ink);
        font-weight: 700;
      }
      .wiz-editorial .ob-link {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        font-family: "Inter", sans-serif;
        font-size: 12.5px;
        font-weight: 600;
        color: var(--ob-purple-deep);
        text-decoration: none;
        border-bottom: 1px solid var(--ob-line-2);
        padding-bottom: 1px;
        margin-top: 4px;
        align-self: flex-start;
      }
      .wiz-editorial .ob-link:hover {
        color: var(--ob-purple);
        border-color: var(--ob-purple);
      }
      @media (max-width: 540px) {
        .wiz-editorial .ob-pillars {
          grid-template-columns: minmax(0, 1fr);
        }
        .wiz-editorial .ob-roles {
          grid-template-columns: minmax(0, 1fr);
        }
        .wiz-editorial .ob-form-card {
          padding: 22px 18px 20px;
          border-radius: 14px;
        }
        .wiz-editorial .ob-cta,
        .wiz-editorial .ob-cta-back,
        .wiz-editorial .ob-cta-ghost {
          width: 100%;
          justify-content: center;
        }
        .wiz-editorial .ob-cta-row.is-split {
          flex-direction: column-reverse;
          align-items: stretch;
        }
      }
      @media (prefers-reduced-motion: reduce) {
        .wiz-editorial .ob-cta,
        .wiz-editorial .ob-cta-back,
        .wiz-editorial .ob-cta-ghost,
        .wiz-editorial .ob-role-card,
        .wiz-editorial .ob-field-input,
        .wiz-editorial .ob-field-textarea {
          transition-duration: 0.001ms !important;
        }
        .wiz-editorial .ob-cta:hover:not(:disabled),
        .wiz-editorial .ob-cta-back:hover:not(:disabled),
        .wiz-editorial .ob-cta-ghost:hover:not(:disabled),
        .wiz-editorial .ob-role-card:hover:not(:disabled) {
          transform: none !important;
        }
      }

      /* "Meet your teammate" single-card preview for first-hire step 5. */
      .wiz-editorial .ob-meet-card {
        display: flex;
        flex-direction: column;
        gap: 20px;
        padding: 4px 0;
      }
      .wiz-editorial .ob-meet-header {
        display: flex;
        align-items: flex-start;
        gap: 18px;
        padding-bottom: 18px;
        border-bottom: 1px solid var(--ob-line);
      }
      .wiz-editorial .ob-meet-avatar {
        width: 56px;
        height: 56px;
        flex-shrink: 0;
        border-radius: 50%;
        background: var(--ob-gradient);
        color: #fff;
        font-family: var(--font-display);
        font-size: 24px;
        display: flex;
        align-items: center;
        justify-content: center;
        letter-spacing: 0.04em;
      }
      .wiz-editorial .ob-meet-who { flex: 1; min-width: 0; }
      .wiz-editorial .ob-meet-name {
        margin: 0 0 4px;
        font-size: 22px;
        display: flex;
        align-items: center;
        gap: 10px;
        flex-wrap: wrap;
      }
      .wiz-editorial .ob-meet-pill {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 3px 10px;
        background: rgba(20, 184, 166, 0.12);
        color: #0f766e;
        border-radius: 999px;
        font-family: var(--font-mono);
        font-size: 10px;
        letter-spacing: 0.12em;
        font-weight: 600;
      }
      .wiz-editorial .ob-meet-pill::before {
        content: "";
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: #14b8a6;
      }
      .wiz-editorial .ob-meet-role {
        margin: 0;
        font-size: 13.5px;
        color: var(--ob-ink-2);
        line-height: 1.55;
      }
      .wiz-editorial .ob-meet-sep {
        color: var(--ob-line);
        margin: 0 8px;
      }
      .wiz-editorial .ob-meet-traits {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
      }
      .wiz-editorial .ob-meet-trait {
        padding: 4px 12px;
        background: linear-gradient(135deg, rgba(139, 92, 246, 0.08), rgba(236, 72, 153, 0.08));
        border: 1px solid rgba(139, 92, 246, 0.18);
        border-radius: 999px;
        font-size: 12px;
        color: var(--ob-ink-1);
      }
      .wiz-editorial .ob-meet-twocol {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 24px;
      }
      @media (max-width: 600px) {
        .wiz-editorial .ob-meet-twocol { grid-template-columns: 1fr; }
      }
      .wiz-editorial .ob-meet-coltitle {
        margin: 0 0 10px;
        font-family: var(--font-mono);
        font-size: 11px;
        letter-spacing: 0.14em;
        text-transform: uppercase;
        font-weight: 700;
        display: flex;
        align-items: center;
        gap: 6px;
      }
      .wiz-editorial .ob-meet-col-great .ob-meet-coltitle { color: #0f766e; }
      .wiz-editorial .ob-meet-col-escalate .ob-meet-coltitle { color: #b45309; }
      .wiz-editorial .ob-meet-col ul {
        list-style: none;
        margin: 0;
        padding: 0;
      }
      .wiz-editorial .ob-meet-col li {
        font-size: 13px;
        color: var(--ob-ink-1);
        padding: 5px 0;
        position: relative;
        padding-left: 14px;
        line-height: 1.45;
      }
      .wiz-editorial .ob-meet-col li::before {
        content: "·";
        position: absolute;
        left: 0;
        color: var(--ob-ink-3);
      }
      .wiz-editorial .ob-meet-signature {
        padding-top: 14px;
        border-top: 1px dashed var(--ob-line);
        font-size: 12.5px;
        color: var(--ob-ink-2);
        line-height: 1.55;
      }
      .wiz-editorial .ob-meet-signature-label {
        font-family: var(--font-mono);
        font-size: 10.5px;
        letter-spacing: 0.16em;
        text-transform: uppercase;
        font-weight: 700;
        color: var(--ob-ink-1);
        margin-right: 8px;
      }
      .wiz-editorial .ob-meet-change {
        margin: 8px 0 0;
        text-align: center;
        font-size: 13px;
        color: var(--ob-ink-3);
      }
      .wiz-editorial .ob-meet-change-link {
        background: none;
        border: none;
        padding: 0;
        color: var(--ob-ink-1);
        text-decoration: underline;
        text-underline-offset: 3px;
        font-size: 13px;
        cursor: pointer;
      }
      .wiz-editorial .ob-meet-change-link:hover { color: var(--ob-purple); }
    `}</style>
  );
}
