import type { ReactNode } from "react";
import { ErrorIllustration, type ErrorIllustrationVariant } from "./ErrorIllustration";

/**
 * Shared layout for the three SPA error surfaces. Centred
 * composition, generous whitespace, max 65ch body width, single
 * inline-SVG motif keyed by `variant`. Headline number uses the
 * --font-display family (Inter Display weights) for a marquee feel
 * that still survives at 320px wide.
 */
export interface ErrorShellProps {
  /** Big numeric or short headline label, e.g. "404" or "Down for maintenance". */
  code: string;
  /** One-line short title under the code. */
  title: string;
  /** One short paragraph; keep under 200 chars. */
  description: string;
  /** Which inline illustration variant to render. */
  variant: ErrorIllustrationVariant;
  /** Optional action row — links/buttons. */
  actions?: ReactNode;
}

export function ErrorShell({ code, title, description, variant, actions }: ErrorShellProps) {
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
        {actions ? <div className="pt-2 flex flex-wrap items-center justify-center gap-3">{actions}</div> : null}
      </div>
    </main>
  );
}
