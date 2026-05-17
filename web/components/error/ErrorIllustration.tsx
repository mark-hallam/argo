import type { JSX } from "react";

export type ErrorIllustrationVariant = "not-found" | "server-error" | "maintenance";

/**
 * Single inline-SVG motif — concentric geometric line-art —
 * rendered in one of three states. Stroke uses --primary so every
 * tenant theme tints the artwork without per-tenant artwork. No
 * stock astronauts, no rainbow gradients, no external asset.
 */
export function ErrorIllustration({
  variant,
}: {
  variant: ErrorIllustrationVariant;
}): JSX.Element {
  const stroke = "var(--primary)";
  const muted = "var(--muted-foreground)";
  return (
    <svg
      width="160"
      height="160"
      viewBox="0 0 160 160"
      role="img"
      aria-hidden="true"
      focusable="false"
      className="mx-auto"
    >
      <circle cx="80" cy="80" r="62" fill="none" stroke={muted} strokeOpacity="0.25" strokeWidth="1" />
      <circle cx="80" cy="80" r="44" fill="none" stroke={muted} strokeOpacity="0.35" strokeWidth="1" />
      {variant === "not-found" && (
        <g stroke={stroke} strokeWidth="2" strokeLinecap="round" fill="none">
          <path d="M55 80 L75 100 L105 60" strokeOpacity="0.0" />
          <path d="M55 60 L105 100 M105 60 L55 100" />
        </g>
      )}
      {variant === "server-error" && (
        <g stroke={stroke} strokeWidth="2" strokeLinecap="round" fill="none">
          <path d="M80 50 L80 90" />
          <circle cx="80" cy="106" r="2.5" fill={stroke} stroke="none" />
        </g>
      )}
      {variant === "maintenance" && (
        <g stroke={stroke} strokeWidth="2" strokeLinecap="round" fill="none">
          <path d="M62 80 L78 96 L106 62" />
        </g>
      )}
    </svg>
  );
}
