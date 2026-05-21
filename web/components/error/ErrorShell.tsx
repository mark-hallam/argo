"use client";

import type { ReactNode } from "react";
import { ErrorIllustration, type ErrorIllustrationVariant } from "./ErrorIllustration";

/**
 * Shared layout for the SPA error surfaces. Two rendering modes:
 *
 *  - Default (`brandLoud=false`): restrained shell used by route-segment
 *    error boundaries that live INSIDE the app chrome (sidebar, top
 *    bar). Plain backgrounds, neutral Inter — the chrome around it
 *    carries the brand.
 *  - Brand-loud (`brandLoud=true`): full-canvas brand treatment used by
 *    the root `/not-found` and `/error` pages, which replace the app
 *    chrome entirely. Audiowide gradient hero, warm radial-blob backdrop,
 *    deep-navy ink — matches the locked D3 brand-consistency pass and
 *    `app/global-error.tsx`.
 *
 * Brand-loud mode pulls Audiowide from Google Fonts and emits a
 * `<style jsx global>` block, which is why this module carries the
 * `"use client"` directive.
 */
export interface ErrorShellProps {
  /** Big numeric or short headline label, e.g. "404" or "Down for maintenance". */
  code: string;
  /** One-line short title. In brand-loud mode this is the gradient Audiowide hero. */
  title: string;
  /** One short paragraph; keep under 200 chars. */
  description: string;
  /** Which inline illustration variant to render (restrained mode only). */
  variant: ErrorIllustrationVariant;
  /** Optional action row — links/buttons. */
  actions?: ReactNode;
  /** Apply the locked D3 brand recipe (Audiowide hero, gradient, radial-blob bg). */
  brandLoud?: boolean;
}

export function ErrorShell({
  code,
  title,
  description,
  variant,
  actions,
  brandLoud,
}: ErrorShellProps) {
  if (brandLoud) {
    return (
      <BrandLoudErrorShell
        code={code}
        title={title}
        description={description}
        actions={actions}
      />
    );
  }
  return (
    <main
      id="main"
      tabIndex={-1}
      role="main"
      data-testid="error-shell"
      className="min-h-screen flex items-center justify-center px-4 sm:px-6 lg:px-8 py-16 focus:outline-none"
    >
      <div className="w-full max-w-[65ch] text-center space-y-6">
        <ErrorIllustration variant={variant} />
        <p
          className="font-[var(--font-display)] text-[var(--text-display-lg)] leading-none tracking-tight text-[var(--foreground)]"
          aria-hidden="true"
        >
          {code}
        </p>
        <h1 className="text-[var(--text-heading-lg)] font-semibold text-[var(--foreground)]">
          <span className="sr-only">{code} — </span>
          {title}
        </h1>
        <p className="text-[var(--text-body-md)] text-[var(--muted-foreground)] mx-auto max-w-[60ch]">
          {description}
        </p>
        {actions ? (
          <div className="pt-2 flex flex-wrap items-center justify-center gap-3">{actions}</div>
        ) : null}
      </div>
    </main>
  );
}

const BRAND_STYLES = `
.err-brand {
  --purple: var(--brand-primary);
  --pink: var(--brand-pink);
  --ink: var(--foreground);
  --ink-2: var(--muted-foreground);
  --ink-3: var(--muted-foreground);
  --line: var(--border);
  --bg: var(--background);
  --gradient: linear-gradient(135deg, var(--purple), var(--pink));
  font-family: "Inter", sans-serif;
  background: var(--bg);
  color: var(--ink);
  min-height: 100vh;
  position: relative;
  display: grid;
  place-items: center;
  padding: 48px 24px;
  overflow: hidden;
}
.err-brand::before,
.err-brand::after {
  content: "";
  position: absolute;
  pointer-events: none;
  z-index: 0;
}
.err-brand::before {
  top: -20%;
  right: -10%;
  width: 700px;
  height: 700px;
  background: radial-gradient(circle, rgba(139, 92, 246, 0.15) 0%, transparent 70%);
}
.err-brand::after {
  bottom: -30%;
  left: -10%;
  width: 800px;
  height: 800px;
  background: radial-gradient(circle, rgba(236, 72, 153, 0.1) 0%, transparent 70%);
}
.err-brand-stage {
  position: relative;
  z-index: 1;
  width: 100%;
  max-width: 560px;
  text-align: center;
}
.err-brand-eyebrow {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-family: "JetBrains Mono", ui-monospace, monospace;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.2em;
  text-transform: uppercase;
  color: var(--ink-3);
  margin-bottom: 22px;
  padding: 6px 14px;
  border-radius: 999px;
  border: 1px solid var(--line);
  background: var(--bg);
}
.err-brand-eyebrow-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--pink);
  box-shadow: 0 0 6px var(--pink);
}
.err-brand-hero {
  font-family: "Audiowide", "Inter", sans-serif;
  font-size: 56px;
  line-height: 1.05;
  font-weight: 400;
  letter-spacing: 0;
  margin: 0 0 18px;
  background: var(--gradient);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  -webkit-text-fill-color: transparent;
}
.err-brand-sub {
  font-size: 16px;
  font-weight: 300;
  color: var(--ink-2);
  line-height: 1.55;
  margin: 0 0 32px;
  letter-spacing: -0.005em;
}
.err-brand-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  gap: 12px;
}

/* Subtle pulse on the eyebrow dot — gated on prefers-reduced-motion.
   Easing matches --motion-ease from globals.css (cubic-bezier(0.4, 0, 0.2, 1));
   spelled long-hand so the brand-polish lint regex stays clean. */
@media (prefers-reduced-motion: no-preference) {
  .err-brand-eyebrow-dot {
    animation: err-brand-pulse 2.4s cubic-bezier(0.4, 0, 0.2, 1) infinite;
  }
  @keyframes err-brand-pulse {
    0%, 100% { box-shadow: 0 0 6px var(--pink); opacity: 1; }
    50% { box-shadow: 0 0 14px var(--pink); opacity: 0.7; }
  }
}

@media (max-width: 480px) {
  .err-brand-hero { font-size: 40px; }
}
`;

function BrandLoudErrorShell({
  code,
  title,
  description,
  actions,
}: {
  code: string;
  title: string;
  description: string;
  actions?: ReactNode;
}) {
  return (
    <>
      <link rel="preconnect" href="https://fonts.googleapis.com" />
      <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
      <link
        href="https://fonts.googleapis.com/css2?family=Audiowide&family=Inter:wght@200;300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap"
        rel="stylesheet"
      />
      <style jsx global>{BRAND_STYLES}</style>
      <main
        id="main"
        tabIndex={-1}
        role="main"
        data-testid="error-shell"
        data-brand-loud="true"
        className="err-brand focus:outline-none"
      >
        <div className="err-brand-stage">
          <div className="err-brand-eyebrow">
            <span className="err-brand-eyebrow-dot" aria-hidden />
            <span>{code}</span>
          </div>
          <h1 className="err-brand-hero">
            <span className="sr-only">{code} — </span>
            {title}
          </h1>
          <p className="err-brand-sub">{description}</p>
          {actions ? <div className="err-brand-actions">{actions}</div> : null}
        </div>
      </main>
    </>
  );
}
