import { cn } from "@/lib/utils";
import {
  normalize,
  toneFillClass,
  toneStrokeClass,
  type ChartTone,
} from "./utils";

/**
 * Pure-SVG sparkline. Renders a polyline between min and max of
 * `data`, normalised to the chart area, with a small filled circle
 * marking the most recent point. Stroke and dot colours bind to the
 * `--<tone>` CSS variable so the chart re-themes per tenant.
 *
 * Edge cases:
 *   - Empty / single-point data → a horizontal mid-band line.
 *   - All-equal series → a horizontal mid-band line.
 *   - One NaN / non-finite point → that point lands at mid-band
 *     instead of blowing up the y-axis.
 */

const PADDING_Y = 2;
const STROKE_W = 1.5;

export interface KpiSparklineProps {
  /** 5–30 data points; oldest → newest. */
  data: number[];
  tone?: ChartTone;
  /** Defaults to 24. */
  height?: number;
  /** Defaults to 80. */
  width?: number;
  ariaLabel?: string;
  className?: string;
}

function buildPath(
  data: number[],
  width: number,
  height: number,
): { d: string; last: { x: number; y: number } | null } {
  if (data.length === 0) {
    const y = height / 2;
    return { d: `M0,${y} L${width},${y}`, last: null };
  }
  if (data.length === 1) {
    const y = height / 2;
    return {
      d: `M0,${y} L${width},${y}`,
      last: { x: width, y },
    };
  }
  const normalised = normalize(data);
  const usableH = height - PADDING_Y * 2;
  const stepX = width / (data.length - 1);
  let last: { x: number; y: number } = { x: 0, y: height / 2 };
  const d = normalised
    .map((n, i) => {
      const x = i * stepX;
      const y = PADDING_Y + (1 - n) * usableH;
      last = { x, y };
      return `${i === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");
  return { d, last };
}

export function KpiSparkline({
  data,
  tone,
  height = 24,
  width = 80,
  ariaLabel,
  className,
}: KpiSparklineProps) {
  const { d, last } = buildPath(data, width, height);
  const label = ariaLabel ?? "Sparkline trend";
  return (
    <span
      data-testid="kpi-sparkline"
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
          {data.length} data point{data.length === 1 ? "" : "s"}
          {data.length > 0 ? `; latest ${data[data.length - 1]}` : ""}
        </desc>
        <path
          d={d}
          fill="none"
          strokeWidth={STROKE_W}
          strokeLinecap="round"
          strokeLinejoin="round"
          className={toneStrokeClass(tone)}
        />
        {last ? (
          <circle
            cx={last.x}
            cy={last.y}
            r={1.75}
            className={toneFillClass(tone)}
          />
        ) : null}
      </svg>
    </span>
  );
}
