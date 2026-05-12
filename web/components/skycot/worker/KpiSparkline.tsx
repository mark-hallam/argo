import { cn } from "@/lib/utils";
import type { KpiTone } from "@/lib/api/worker-overview";

/**
 * Pure-SVG sparkline primitive. No external chart library — keeps
 * the Overview-tab bundle small (E4-b acceptance criteria: <8KB
 * First-Load delta). 80×24 viewBox; the wrapping span scales to
 * whatever width the parent layout grants.
 *
 * Renders a single polyline path; `tone` maps to the
 * design-system success/warning/muted CSS variables so the line
 * inherits the active theme.
 *
 * Edge cases:
 *   - Empty / single-point series → renders the line at midheight.
 *   - All-equal series (zero amplitude) → renders a horizontal line.
 *   - One NaN / non-finite point → clamped to the series midpoint
 *     so a single bad reading doesn't blow up the y-axis.
 */

const VIEWBOX_W = 80;
const VIEWBOX_H = 24;
const STROKE_W = 1.5;
const PADDING_Y = 2;

export interface KpiSparklineProps {
  series: number[];
  tone?: KpiTone;
  /** Aria-label override; defaults to "Sparkline trend". */
  ariaLabel?: string;
  className?: string;
}

function sanitize(series: number[]): number[] {
  if (series.length === 0) return [];
  const finite = series.filter((n) => Number.isFinite(n));
  if (finite.length === 0) return series.map(() => 0);
  const mid = finite[Math.floor(finite.length / 2)] ?? 0;
  return series.map((n) => (Number.isFinite(n) ? n : mid));
}

function toPathPoints(series: number[]): string {
  const clean = sanitize(series);
  if (clean.length === 0) {
    const y = VIEWBOX_H / 2;
    return `M0,${y} L${VIEWBOX_W},${y}`;
  }
  if (clean.length === 1) {
    const y = VIEWBOX_H / 2;
    return `M0,${y} L${VIEWBOX_W},${y}`;
  }
  const min = Math.min(...clean);
  const max = Math.max(...clean);
  const span = max - min || 1;
  const usableH = VIEWBOX_H - PADDING_Y * 2;
  const stepX = VIEWBOX_W / (clean.length - 1);

  return clean
    .map((v, i) => {
      const x = i * stepX;
      const normalised = (v - min) / span;
      const y = PADDING_Y + (1 - normalised) * usableH;
      return `${i === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");
}

function toneStrokeClass(tone: KpiTone | undefined): string {
  switch (tone) {
    case "success":
      return "stroke-[var(--success)]";
    case "warning":
      return "stroke-[var(--warning)]";
    case "muted":
    default:
      return "stroke-[var(--muted-foreground)]";
  }
}

export function KpiSparkline({
  series,
  tone,
  ariaLabel,
  className,
}: KpiSparklineProps) {
  const d = toPathPoints(series);
  return (
    <span
      data-testid="kpi-sparkline"
      data-tone={tone ?? "muted"}
      className={cn("inline-block align-middle", className)}
    >
      <svg
        viewBox={`0 0 ${VIEWBOX_W} ${VIEWBOX_H}`}
        width={VIEWBOX_W}
        height={VIEWBOX_H}
        role="img"
        aria-label={ariaLabel ?? "Sparkline trend"}
        preserveAspectRatio="none"
        className="block"
      >
        <path
          d={d}
          fill="none"
          strokeWidth={STROKE_W}
          strokeLinecap="round"
          strokeLinejoin="round"
          className={toneStrokeClass(tone)}
        />
      </svg>
    </span>
  );
}
