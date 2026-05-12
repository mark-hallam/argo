import { cn } from "@/lib/utils";
import { normalize, toneFillClass, type ChartTone } from "./utils";

/**
 * Vertical bar chart — typically 5–7 bars in a 32×24 footprint for
 * week-over-week / daily-rate visuals next to a KPI value. Bars
 * share a single tone; height encodes the normalised value, with a
 * 1px minimum so a zero / floor bar is still visible.
 *
 * Edge cases mirror KpiSparkline:
 *   - Empty data → no bars (just an aria-labelled SVG container).
 *   - All-equal series → uniform mid-height bars.
 *   - Non-finite values → drawn at the mid-band.
 */

const GAP = 1;
const MIN_BAR_H = 1;

export interface MiniBarProps {
  data: number[];
  tone?: ChartTone;
  /** Defaults to 24. */
  height?: number;
  /** Defaults to 32. */
  width?: number;
  ariaLabel?: string;
  className?: string;
}

function barLayout(data: number[], width: number, height: number) {
  if (data.length === 0) return [] as Array<{ x: number; y: number; w: number; h: number }>;
  const normalised = normalize(data);
  const totalGap = GAP * (data.length - 1);
  const barW = Math.max(1, (width - totalGap) / data.length);
  return normalised.map((n, i) => {
    const h = Math.max(MIN_BAR_H, n * height);
    return {
      x: i * (barW + GAP),
      y: height - h,
      w: barW,
      h,
    };
  });
}

export function MiniBar({
  data,
  tone,
  height = 24,
  width = 32,
  ariaLabel,
  className,
}: MiniBarProps) {
  const bars = barLayout(data, width, height);
  const label = ariaLabel ?? "Bar chart";
  return (
    <span
      data-testid="mini-bar"
      data-tone={tone ?? "muted"}
      className={cn("inline-block align-middle", className)}
    >
      <svg
        viewBox={`0 0 ${width} ${height}`}
        width={width}
        height={height}
        role="img"
        aria-label={label}
        preserveAspectRatio="none"
        className="block"
      >
        <title>{label}</title>
        <desc>
          {data.length} bar{data.length === 1 ? "" : "s"}
          {data.length > 0 ? `; latest ${data[data.length - 1]}` : ""}
        </desc>
        {bars.map((b, i) => (
          <rect
            key={i}
            x={b.x}
            y={b.y}
            width={b.w}
            height={b.h}
            rx={0.5}
            className={toneFillClass(tone)}
            data-testid={`mini-bar-bar-${i}`}
          />
        ))}
      </svg>
    </span>
  );
}
