"use client";

import Link from "next/link";
import type { ReactNode } from "react";

/**
 * Locked editorial-A auth shell (2026-05-18 design lock).
 *
 * Provides the SKYCOT brand language for every auth surface:
 *  - Audiowide hero typography via Google Fonts
 *  - Purple→pink→coral gradient on emphasized text and CTA
 *  - Warm radial-blob page background (var(--background) with purple + pink blooms)
 *  - Centered editorial stage (max-width 460px)
 *  - Header with brand mark + right-side CTA link
 *  - Optional footer-trust strip (SOC 2 / GDPR / SSO READY)
 *
 * Used by /auth/signup, /auth/signin, /auth/check-email, /auth/verify so
 * every step of the auth flow stays brand-consistent. Children render
 * inside the centered stage; use the included class names (eyebrow,
 * hero, sub, field-block, field-label, field-input, cta, etc.) for
 * automatic brand styling.
 */
export function AuthEditorialShell({
  children,
  topRightLabel,
  topRightHref,
  topRightCta,
  showTrustFoot = true,
}: {
  children: ReactNode;
  topRightLabel: string;
  topRightHref: string;
  topRightCta: string;
  showTrustFoot?: boolean;
}) {
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{`
        .auth-editorial {
          --purple: var(--brand-primary);
          --purple-deep: #6d28d9;
          --pink: var(--brand-pink);
          --coral: var(--brand-coral);
          --teal: var(--brand-teal);
          --ink: var(--foreground);
          --ink-2: var(--muted-foreground);
          --ink-3: var(--muted-foreground);
          --ink-4: var(--muted-foreground);
          --bg: var(--background);
          --line: var(--border);
          --line-2: var(--border);
          --gradient: linear-gradient(135deg, var(--purple), var(--pink));
          --gradient-3: linear-gradient(135deg, var(--purple), var(--pink), var(--coral));
          font-family: "Inter", sans-serif;
          background: var(--bg);
          color: var(--ink);
          font-size: 16px;
          line-height: 1.55;
          -webkit-font-smoothing: antialiased;
          letter-spacing: -0.005em;
          min-height: 100vh;
          position: relative;
        }
        .auth-editorial::before {
          content: "";
          position: fixed;
          top: -20%;
          right: -10%;
          width: 700px;
          height: 700px;
          background: radial-gradient(circle, rgba(139, 92, 246, 0.15) 0%, transparent 70%);
          z-index: 0;
          pointer-events: none;
        }
        .auth-editorial::after {
          content: "";
          position: fixed;
          bottom: -30%;
          left: -10%;
          width: 800px;
          height: 800px;
          background: radial-gradient(circle, rgba(236, 72, 153, 0.1) 0%, transparent 70%);
          z-index: 0;
          pointer-events: none;
        }
        .auth-top,
        .auth-stage {
          position: relative;
          z-index: 1;
        }
        .auth-top {
          padding: 22px 48px;
          display: flex;
          align-items: center;
          justify-content: space-between;
        }
        .auth-brand {
          display: flex;
          align-items: center;
          gap: 12px;
        }
        .auth-brand-mark {
          width: 28px;
          height: 28px;
          border-radius: 7px;
          background: var(--gradient);
          box-shadow: 0 4px 12px -2px rgba(139, 92, 246, 0.3);
        }
        .auth-brand-name {
          font-family: "Audiowide", sans-serif;
          font-size: 16px;
          letter-spacing: 0.08em;
          color: var(--ink);
        }
        .auth-top-right {
          display: flex;
          align-items: center;
          gap: 16px;
          font-size: 13px;
          color: var(--ink-3);
        }
        .auth-top-right a {
          color: var(--ink);
          font-weight: 600;
          border-bottom: 1px solid var(--line-2);
          padding-bottom: 1px;
          text-decoration: none;
        }
        .auth-stage {
          max-width: 460px;
          margin: 0 auto;
          padding: 64px 48px 100px;
        }
        .eyebrow {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-3);
          letter-spacing: 0.2em;
          text-transform: uppercase;
          font-weight: 700;
          margin-bottom: 22px;
          padding: 5px 14px;
          border-radius: 999px;
          border: 1px solid var(--line);
          background: var(--bg);
        }
        .eyebrow-dot {
          width: 6px;
          height: 6px;
          border-radius: 50%;
          background: var(--teal);
          box-shadow: 0 0 6px var(--teal);
        }
        .hero {
          font-family: "Audiowide", "Inter", sans-serif;
          font-size: 56px;
          line-height: 1.05;
          font-weight: 400;
          letter-spacing: 0;
          margin: 0 0 18px;
        }
        .hero strong {
          font-weight: 400;
          background: var(--gradient-3);
          -webkit-background-clip: text;
          background-clip: text;
          color: transparent;
        }
        .hero-small {
          font-size: 42px;
        }
        .sub {
          font-size: 17px;
          font-weight: 300;
          color: var(--ink-2);
          line-height: 1.5;
          margin: 0 0 36px;
        }
        .sub .emphasis,
        .sub strong {
          font-weight: 600;
          color: var(--ink);
        }
        .sso-row {
          display: flex;
          flex-direction: column;
          gap: 10px;
          margin-bottom: 24px;
        }
        .sso-btn {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 10px;
          padding: 13px 18px;
          border-radius: 10px;
          border: 1px solid var(--line);
          background: var(--bg);
          color: var(--ink);
          font-family: "Inter", sans-serif;
          font-size: 14.5px;
          font-weight: 600;
          cursor: not-allowed;
          opacity: 0.7;
          position: relative;
        }
        .sso-btn:hover {
          border-color: var(--line-2);
        }
        .sso-btn .logo {
          width: 18px;
          height: 18px;
          border-radius: 4px;
          display: inline-flex;
          align-items: center;
          justify-content: center;
        }
        .sso-btn .logo svg {
          width: 18px;
          height: 18px;
        }
        .sso-btn .coming-soon {
          position: absolute;
          right: 14px;
          font-family: "JetBrains Mono", monospace;
          font-size: 9.5px;
          color: var(--ink-3);
          letter-spacing: 0.08em;
        }
        .sso-btn-active {
          cursor: pointer;
          opacity: 1;
          transition: border-color 0.15s, box-shadow 0.15s, transform 0.05s;
        }
        .sso-btn-active:hover {
          border-color: var(--purple);
          box-shadow: 0 4px 14px -6px rgba(139, 92, 246, 0.4);
        }
        .sso-btn-active:active {
          transform: translateY(1px);
        }
        .sso-btn-active:disabled {
          cursor: not-allowed;
          opacity: 0.55;
        }
        .sso-btn-highlight {
          border-color: var(--purple);
          background: linear-gradient(135deg, rgba(139, 92, 246, 0.06), rgba(236, 72, 153, 0.06));
          box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.08);
        }
        .divider {
          display: flex;
          align-items: center;
          gap: 14px;
          margin: 20px 0;
        }
        .divider::before,
        .divider::after {
          content: "";
          flex: 1;
          height: 1px;
          background: var(--line);
        }
        .divider-lbl {
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: var(--ink-3);
          text-transform: uppercase;
          letter-spacing: 0.16em;
          font-weight: 700;
        }
        .field-block {
          margin-bottom: 16px;
        }
        .field-label {
          display: block;
          font-family: "JetBrains Mono", monospace;
          font-size: 10.5px;
          color: var(--ink-3);
          text-transform: uppercase;
          letter-spacing: 0.16em;
          font-weight: 700;
          margin-bottom: 8px;
        }
        .field-input {
          width: 100%;
          padding: 14px 18px;
          border-radius: 10px;
          border: 1px solid var(--line);
          background: var(--bg);
          font-family: "Inter", sans-serif;
          font-size: 16px;
          font-weight: 500;
          color: var(--ink);
          letter-spacing: -0.005em;
          outline: none;
          transition: border-color 0.15s, box-shadow 0.15s;
        }
        .field-input:focus {
          border-color: var(--purple);
          box-shadow: 0 0 0 4px rgba(139, 92, 246, 0.1);
        }
        .form-error {
          font-size: 13px;
          color: var(--coral);
          margin: -8px 0 8px;
        }
        .cta {
          width: 100%;
          padding: 15px 22px;
          border-radius: 12px;
          background: var(--gradient);
          color: white;
          border: none;
          font-size: 15px;
          font-weight: 700;
          cursor: pointer;
          box-shadow: 0 6px 20px -4px rgba(139, 92, 246, 0.45);
          margin-top: 6px;
          letter-spacing: -0.005em;
          display: flex;
          align-items: center;
          justify-content: center;
          gap: 8px;
          font-family: "Inter", sans-serif;
        }
        .cta:disabled {
          opacity: 0.6;
          cursor: not-allowed;
        }
        .cta-secondary {
          background: transparent;
          color: var(--ink);
          border: 1px solid var(--line-2);
          box-shadow: none;
        }
        .cta .kbd {
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          padding: 2px 7px;
          border-radius: 5px;
          background: rgba(255, 255, 255, 0.25);
        }
        .below-form {
          margin-top: 18px;
          font-size: 13px;
          color: var(--ink-3);
          text-align: center;
        }
        .trouble {
          margin-top: 36px;
          padding-top: 24px;
          border-top: 1px solid var(--line);
          text-align: center;
          font-size: 12.5px;
          color: var(--ink-3);
        }
        .trouble a {
          color: var(--ink-2);
          text-decoration: none;
          border-bottom: 1px solid var(--line-2);
          padding-bottom: 1px;
          font-weight: 600;
        }
        .foot {
          margin-top: 28px;
          display: flex;
          gap: 18px;
          align-items: center;
          justify-content: center;
          font-family: "JetBrains Mono", monospace;
          font-size: 11px;
          color: var(--ink-4);
          letter-spacing: 0.06em;
          text-transform: uppercase;
          font-weight: 600;
        }
        .foot-item {
          display: inline-flex;
          gap: 5px;
          align-items: center;
        }
        .foot-dot {
          width: 5px;
          height: 5px;
          border-radius: 50%;
          background: var(--teal);
        }
        @media (max-width: 540px) {
          .auth-top {
            padding: 16px 24px;
          }
          .auth-stage {
            padding: 32px 24px 80px;
          }
          .hero {
            font-size: 42px;
          }
        }
        /* A11y: respect prefers-reduced-motion (WCAG 2.3.3). Auth pages
           do not mount MotionRoot, so framer-motion's reduced-motion
           swap doesn't reach this subtree. Neutralize all animations
           and shorten transitions inside the auth shell so radial
           blobs, hover transforms, and any future keyframes freeze for
           users who set the OS preference. */
        @media (prefers-reduced-motion: reduce) {
          .auth-editorial,
          .auth-editorial *,
          .auth-editorial *::before,
          .auth-editorial *::after {
            animation-duration: 0.001ms !important;
            animation-iteration-count: 1 !important;
            transition-duration: 0.001ms !important;
            scroll-behavior: auto !important;
          }
        }
      `}</style>
      <div className="auth-editorial">
        <header className="auth-top">
          <div className="auth-brand">
            <div className="auth-brand-mark" />
            <div className="auth-brand-name">SKYCOT</div>
          </div>
          <div className="auth-top-right">
            <span>{topRightLabel}</span>
            <Link href={topRightHref}>{topRightCta}</Link>
          </div>
        </header>
        <main id="main" tabIndex={-1} className="auth-stage">
          {children}
          {showTrustFoot ? (
            <div className="foot">
              <span className="foot-item"><span className="foot-dot" />SOC 2-READY AUDIT TRAIL</span>
              <span className="foot-item"><span className="foot-dot" />GDPR</span>
              <span className="foot-item"><span className="foot-dot" />SSO READY</span>
            </div>
          ) : null}
        </main>
      </div>
    </>
  );
}

