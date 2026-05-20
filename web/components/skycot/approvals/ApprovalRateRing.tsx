"use client";

import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

/**
 * Rolling 7-day approval-rate ring shown in the KPI strip.
 * Renders three slices — approved / refused / expired — sourced from
 * the gateway's `/api/v1/approvals/stats` rollup. Defaults to a static
 * frame; pass `animate` to sweep the conic-gradient in on mount.
 *
 * The ring shows the *approval* slice as the headline number in the
 * center. The full distribution sits in the legend on the right of the
 * KPI strip via the `slices` prop.
 *
 * Honors `prefers-reduced-motion` — users with the OS preference set
 * see the final frame immediately rather than the conic sweep.
 */

function prefersReducedMotion(): boolean {
  if (typeof window === "undefined") return false;
  if (typeof window.matchMedia !== "function") return false;
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export interface ApprovalRateRingProps {
  /** Approved share in [0, 1]. */
  approved: number;
  /** Refused share in [0, 1]. */
  refused: number;
  /** Expired / snoozed share in [0, 1]. */
  expired: number;
  size?: number;
  animate?: boolean;
  className?: string;
}

export function ApprovalRateRing({
  approved,
  refused,
  expired,
  size = 64,
  animate = false,
  className,
}: ApprovalRateRingProps) {
  // Defer the animation start to the client so SSR + CSR render the
  // same static frame; the transition only fires after hydration.
  // Reduced-motion users jump straight to the filled frame.
  const [mounted, setMounted] = useState(false);
  useEffect(() => {
    if (!animate) {
      setMounted(true);
      return;
    }
    if (prefersReducedMotion()) {
      setMounted(true);
      return;
    }
    const id = window.requestAnimationFrame(() => setMounted(true));
    return () => window.cancelAnimationFrame(id);
  }, [animate]);

  const total = Math.max(0, approved + refused + expired);
  const a = total === 0 ? 0 : approved / total;
  const r = total === 0 ? 0 : refused / total;
  const e = total === 0 ? 0 : expired / total;

  const aStop = `${(a * 100).toFixed(2)}%`;
  const rStop = `${((a + r) * 100).toFixed(2)}%`;

  const gradient = `conic-gradient(var(--success) 0 ${aStop}, var(--destructive) ${aStop} ${rStop}, var(--muted-foreground) ${rStop} 100%)`;

  const emptyGradient = `conic-gradient(var(--muted) 0 100%)`;
  const showFinal = !animate || mounted;

  const innerSize = Math.max(20, Math.round(size * 0.62));

  return (
    <span
      data-testid="approval-rate-ring"
      data-approved={Math.round(a * 100)}
      data-refused={Math.round(r * 100)}
      data-expired={Math.round(e * 100)}
      className={cn(
        "relative inline-grid place-items-center rounded-full",
        className,
      )}
      style={{
        width: size,
        height: size,
        background: showFinal ? gradient : emptyGradient,
        transition:
          animate && !prefersReducedMotion()
            ? "background 600ms var(--motion-ease)"
            : undefined,
      }}
    >
      <span
        aria-hidden
        className="grid place-items-center rounded-full bg-[var(--card)]"
        style={{ width: innerSize, height: innerSize }}
      >
        <span className="text-[12px] font-bold tabular-nums text-[var(--foreground)]">
          {Math.round(a * 100)}%
        </span>
      </span>
    </span>
  );
}
