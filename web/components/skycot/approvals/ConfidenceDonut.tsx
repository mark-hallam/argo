"use client";

import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

/**
 * Worker self-reported confidence donut.
 *  - high  (>=85%)  → success / teal
 *  - mid   (>=60%)  → warning / amber
 *  - low   (<60%)   → destructive / coral
 *
 * Rendered with a CSS `conic-gradient` so no extra layout passes are
 * required. Pure presentation — animation is opt-in via the
 * `animate` prop, which adds a one-shot CSS transition. If the user
 * has `prefers-reduced-motion: reduce` set we skip straight to the
 * final frame (WCAG 2.3.3 — no animation past 5 seconds and no
 * gratuitous animation at all). Mirrors the BrandLoudHero pattern.
 */

function prefersReducedMotion(): boolean {
  if (typeof window === "undefined") return false;
  if (typeof window.matchMedia !== "function") return false;
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export interface ConfidenceDonutProps {
  /** Confidence in [0, 1]; values outside the range are clamped. */
  value: number;
  /** Size in pixels (square). Defaults to 26. */
  size?: number;
  /** Animate the conic-gradient on mount (respects reduced-motion). */
  animate?: boolean;
  /** Show the percent label next to the donut. Default true. */
  showLabel?: boolean;
  className?: string;
}

function tone(value: number): "high" | "mid" | "low" {
  if (value >= 0.85) return "high";
  if (value >= 0.6) return "mid";
  return "low";
}

const TONE_VAR: Record<"high" | "mid" | "low", string> = {
  high: "var(--success)",
  mid: "var(--warning)",
  low: "var(--destructive)",
};

export function ConfidenceDonut({
  value,
  size = 26,
  animate = false,
  showLabel = true,
  className,
}: ConfidenceDonutProps) {
  const clamped = Math.max(0, Math.min(1, value));
  const pct = Math.round(clamped * 100);
  const t = tone(clamped);
  const filledPct = `${clamped * 100}%`;

  // For the animate path: render an empty ring on first paint, then
  // swap to the filled gradient after mount so the conic-gradient
  // visibly fills in. Reduced-motion users skip straight to the
  // final frame.
  const [filled, setFilled] = useState<boolean>(!animate);
  useEffect(() => {
    if (!animate) {
      setFilled(true);
      return;
    }
    if (prefersReducedMotion()) {
      setFilled(true);
      return;
    }
    const id = window.requestAnimationFrame(() => setFilled(true));
    return () => window.cancelAnimationFrame(id);
  }, [animate]);

  const ringStyle: React.CSSProperties = {
    width: size,
    height: size,
    background: filled
      ? `conic-gradient(${TONE_VAR[t]} 0 ${filledPct}, var(--muted) ${filledPct} 100%)`
      : `conic-gradient(${TONE_VAR[t]} 0 0%, var(--muted) 0% 100%)`,
    transition:
      animate && !prefersReducedMotion()
        ? "background 480ms var(--motion-ease)"
        : undefined,
  };

  const innerSize = Math.max(8, Math.round(size * 0.55));

  return (
    <span
      data-testid="confidence-donut"
      data-tone={t}
      data-percent={pct}
      className={cn("inline-flex items-center gap-2", className)}
    >
      <span
        aria-hidden
        className="relative inline-block rounded-full"
        style={ringStyle}
      >
        <span
          className="absolute rounded-full bg-[var(--card)]"
          style={{
            width: innerSize,
            height: innerSize,
            top: "50%",
            left: "50%",
            transform: "translate(-50%, -50%)",
          }}
        />
      </span>
      {showLabel ? (
        <span className="inline-flex flex-col leading-none">
          <span className="font-mono text-[11.5px] font-semibold text-[var(--foreground)] tabular-nums">
            {pct}%
          </span>
          <span className="font-mono text-[9.5px] text-[var(--muted-foreground)] mt-0.5">
            conf
          </span>
        </span>
      ) : (
        <span className="sr-only">{pct}% confidence</span>
      )}
    </span>
  );
}
