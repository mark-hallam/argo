/**
 * Shared helpers for the chart primitives.
 *
 * No external charting library; the primitives are pure SVG. These
 * helpers cover the two pieces of arithmetic every primitive needs:
 * normalising a series into the 0–1 chart-area range, and mapping a
 * numeric value onto a semantic tone via threshold rules.
 *
 * `ChartTone` is the canonical tone vocabulary for the charts
 * directory. It is a superset of `KpiTone` from
 * `@/lib/api/worker-overview`; passing a KpiTone here is type-safe.
 *
 * Tone-class helpers return literal Tailwind class names (no string
 * interpolation) so the Tailwind v4 scanner picks them up at build.
 */

export type ChartTone =
  | "success"
  | "warning"
  | "muted"
  | "destructive"
  | "info";

export function normalize(
  data: number[],
  min?: number,
  max?: number,
): number[] {
  if (data.length === 0) return [];
  const finite = data.filter((v) => Number.isFinite(v));
  if (finite.length === 0) return data.map(() => 0.5);
  const lo = min ?? Math.min(...finite);
  const hi = max ?? Math.max(...finite);
  const range = hi - lo;
  if (range === 0) return data.map(() => 0.5);
  return data.map((v) => (Number.isFinite(v) ? (v - lo) / range : 0.5));
}

export function thresholdTone(
  value: number,
  thresholds: { warning?: number; destructive?: number },
): "muted" | "warning" | "destructive" {
  if (thresholds.destructive != null && value >= thresholds.destructive) {
    return "destructive";
  }
  if (thresholds.warning != null && value >= thresholds.warning) {
    return "warning";
  }
  return "muted";
}

const STROKE: Record<ChartTone, string> = {
  success: "stroke-[var(--success)]",
  warning: "stroke-[var(--warning)]",
  muted: "stroke-[var(--muted-foreground)]",
  destructive: "stroke-[var(--destructive)]",
  info: "stroke-[var(--info)]",
};
const FILL: Record<ChartTone, string> = {
  success: "fill-[var(--success)]",
  warning: "fill-[var(--warning)]",
  muted: "fill-[var(--muted-foreground)]",
  destructive: "fill-[var(--destructive)]",
  info: "fill-[var(--info)]",
};
const TEXT: Record<ChartTone, string> = {
  success: "text-[var(--success)]",
  warning: "text-[var(--warning)]",
  muted: "text-[var(--muted-foreground)]",
  destructive: "text-[var(--destructive)]",
  info: "text-[var(--info)]",
};

export const toneStrokeClass = (t: ChartTone | undefined) => STROKE[t ?? "muted"];
export const toneFillClass = (t: ChartTone | undefined) => FILL[t ?? "muted"];
export const toneTextClass = (t: ChartTone | undefined) => TEXT[t ?? "muted"];
