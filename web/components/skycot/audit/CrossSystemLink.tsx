/**
 * `<CrossSystemLink>` — inline anchor that renders a detected
 * cross-system reference with a small `↗` icon, a system tooltip,
 * and the strict `target="_blank" rel="noopener noreferrer"`
 * attribute set required for open-in-new-tab safety.
 *
 * Visual contract: raw matched text + `↗` icon (12px). On hover,
 * an underline appears under the text only; the icon stays inert.
 * Color is the design-system `--primary` token so it tracks the
 * tenant theme.
 *
 * The component is intentionally policy-free — it does not check
 * `verifyOrigin`. The detection layer (`lib/cross-system/detect`)
 * filters out any generator output that fails its connector's
 * origin allowlist, so any URL reaching the component has already
 * been validated.
 */
"use client";

import { ExternalLink } from "lucide-react";

import type { ConnectorSystem, RenderedCrossSystemLink } from "@/lib/cross-system/types";

export interface CrossSystemLinkProps {
  /** Connector this link belongs to — used for the testid + tooltip. */
  system: ConnectorSystem;
  /** Human-readable connector name for the tooltip. */
  systemLabel: string;
  /** Validated deep-link URL. */
  href: string;
  /** Raw matched text the link wraps. */
  children: string;
  /** Optional accessible label override; defaults to "Opens in <System>". */
  ariaLabel?: string;
}

export function CrossSystemLink({
  system,
  systemLabel,
  href,
  children,
  ariaLabel,
}: CrossSystemLinkProps) {
  const tooltip = `Opens in ${systemLabel}`;
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      title={tooltip}
      aria-label={ariaLabel ?? tooltip}
      data-testid={`audit-output-cross-link-${system}`}
      data-cross-system={system}
      className="inline-flex items-baseline gap-0.5 text-[var(--primary)] underline-offset-2 hover:underline"
    >
      <span>{children}</span>
      <ExternalLink aria-hidden className="size-3 translate-y-[1px]" />
    </a>
  );
}

/** Build the props the renderer passes to `<CrossSystemLink>` from a
 *  detected match. Pure function — kept here so callers can compose
 *  custom renderers without duplicating mapping logic. */
export function renderPropsFromMatch(
  match: RenderedCrossSystemLink,
): CrossSystemLinkProps {
  return {
    system: match.system,
    systemLabel: match.systemLabel,
    href: match.url,
    children: match.label,
  };
}
