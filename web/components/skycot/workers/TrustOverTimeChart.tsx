"use client";

import { useId, useMemo, useState } from "react";
import type { TrustGradeId } from "@/lib/api/worker-trust";
import type { TrustHistoryPoint } from "@/lib/sample-worker-trust-history";

/**
 * Read-only line chart of a worker's trust-grade over time.
 *
 * Renders a raw SVG path (the workspace has no chart dep — see
 * `web/package.json`) and shows a date+value tooltip on hover.
 * The Y axis is the 5-rung trust ladder; the X axis is the
 * window covered by the `points` array (typically 90 days).
 *
 * Animation: a single 350ms `stroke-dashoffset` reveal on mount.
 * Wrapped in a `prefers-reduced-motion: reduce` guard per WCAG
 * 2.3.3, matching the rest of the SKYCOT design system.
 */

const GRADE_LABELS: Record<TrustGradeId, string> = {
  learned_unreviewed: "Learned",
  published_community: "Community",
  tenant: "Tenant",
  published_verified: "Verified",
  builtin: "Builtin",
};

const VALUE_GRADE: TrustGradeId[] = [
  "learned_unreviewed",
  "published_community",
  "tenant",
  "published_verified",
  "builtin",
];

const VIEW = { width: 720, height: 220, padTop: 16, padBottom: 32, padLeft: 56, padRight: 16 };
const Y_MIN = 0;
const Y_MAX = 4;

function fmtDate(ts: number): string {
  return new Date(ts).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function gradeOf(value: number): TrustGradeId {
  const clamped = Math.max(0, Math.min(VALUE_GRADE.length - 1, Math.round(value)));
  return VALUE_GRADE[clamped]!;
}

export interface TrustOverTimeChartProps {
  /** Time-ordered series of points (oldest first). */
  points: TrustHistoryPoint[];
  /** Used as the chart's `aria-label` so screen readers know whose history they're hearing. */
  workerName?: string;
  className?: string;
}

export function TrustOverTimeChart({
  points,
  workerName,
  className,
}: TrustOverTimeChartProps) {
  const gradId = useId();
  const [hoverIdx, setHoverIdx] = useState<number | null>(null);

  const layout = useMemo(() => {
    if (points.length === 0) return null;
    const plotW = VIEW.width - VIEW.padLeft - VIEW.padRight;
    const plotH = VIEW.height - VIEW.padTop - VIEW.padBottom;
    const xFor = (i: number) =>
      VIEW.padLeft +
      (points.length === 1 ? plotW / 2 : (i / (points.length - 1)) * plotW);
    const yFor = (v: number) =>
      VIEW.padTop + plotH - ((v - Y_MIN) / (Y_MAX - Y_MIN)) * plotH;
    const coords = points.map((p, i) => ({ x: xFor(i), y: yFor(p.value), point: p }));
    const linePath = coords
      .map(({ x, y }, i) => `${i === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`)
      .join(" ");
    const areaPath =
      `${linePath} L${coords.at(-1)!.x.toFixed(2)},${VIEW.padTop + plotH} ` +
      `L${coords[0]!.x.toFixed(2)},${VIEW.padTop + plotH} Z`;
    return { plotW, plotH, xFor, yFor, coords, linePath, areaPath };
  }, [points]);

  if (!layout || points.length === 0) {
    return (
      <div
        data-testid="trust-over-time-empty"
        className={className}
        role="img"
        aria-label="No trust-grade history yet"
      >
        <p className="text-sm text-[var(--muted-foreground)]">
          No trust-grade history yet — this worker hasn&apos;t accumulated a
          90-day signal.
        </p>
      </div>
    );
  }

  const handleMove = (event: React.MouseEvent<SVGSVGElement>) => {
    const rect = event.currentTarget.getBoundingClientRect();
    const xLocal = ((event.clientX - rect.left) / rect.width) * VIEW.width;
    const fraction = Math.max(
      0,
      Math.min(1, (xLocal - VIEW.padLeft) / (VIEW.width - VIEW.padLeft - VIEW.padRight)),
    );
    const idx = Math.round(fraction * (points.length - 1));
    setHoverIdx(idx);
  };

  const handleKey = (event: React.KeyboardEvent<SVGSVGElement>) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const delta = event.key === "ArrowRight" ? 1 : -1;
    const base = hoverIdx ?? points.length - 1;
    const next = Math.max(0, Math.min(points.length - 1, base + delta));
    setHoverIdx(next);
  };

  const hover = hoverIdx !== null ? layout.coords[hoverIdx] : null;
  const first = layout.coords[0]!;
  const last = layout.coords.at(-1)!;
  const firstLabel = fmtDate(first.point.ts);
  const lastLabel = fmtDate(last.point.ts);
  const ariaLabel = workerName
    ? `Trust grade over time for ${workerName}: ${GRADE_LABELS[first.point.grade]} on ${firstLabel} to ${GRADE_LABELS[last.point.grade]} on ${lastLabel}`
    : "Trust grade over time";

  return (
    <div className={className} data-testid="trust-over-time-chart">
      <style jsx>{`
        .toc-card {
          background: white;
          border: 1px solid var(--border, #ece9f5);
          border-radius: 14px;
          padding: 22px;
          position: relative;
          overflow: hidden;
        }
        .toc-header {
          display: flex;
          align-items: baseline;
          justify-content: space-between;
          gap: 12px;
          margin-bottom: 4px;
        }
        .toc-title {
          font-size: 13.5px;
          font-weight: 600;
          color: var(--foreground, #1e1b4b);
          margin: 0;
        }
        .toc-meta {
          font-size: 11.5px;
          color: var(--muted-foreground, #52525b);
          letter-spacing: 0.04em;
        }
        .toc-svg-wrap {
          margin-top: 12px;
          position: relative;
        }
        .toc-svg {
          width: 100%;
          height: auto;
          display: block;
          touch-action: pan-y;
        }
        .toc-line {
          fill: none;
          stroke: #8b5cf6;
          stroke-width: 2;
          stroke-linecap: round;
          stroke-linejoin: round;
          stroke-dasharray: 1600;
          stroke-dashoffset: 1600;
          animation: toc-reveal 0.45s var(--motion-ease, cubic-bezier(0.4, 0, 0.2, 1)) forwards;
        }
        .toc-area {
          fill: url(#${gradId});
        }
        .toc-dot {
          fill: white;
          stroke: #8b5cf6;
          stroke-width: 2;
        }
        .toc-axis text,
        .toc-axis line {
          font-family: ui-sans-serif, system-ui, sans-serif;
          font-size: 10.5px;
          fill: var(--muted-foreground, #52525b);
          stroke: var(--border, #ece9f5);
        }
        .toc-tooltip {
          position: absolute;
          background: var(--foreground, #1e1b4b);
          color: white;
          font-size: 11.5px;
          padding: 6px 10px;
          border-radius: 8px;
          pointer-events: none;
          transform: translate(-50%, -130%);
          white-space: nowrap;
          box-shadow: 0 4px 16px rgba(30, 27, 75, 0.18);
        }
        .toc-tooltip-grade {
          font-weight: 700;
          color: #c4b5fd;
          letter-spacing: 0.04em;
          text-transform: uppercase;
          font-size: 10px;
        }
        @media (prefers-reduced-motion: reduce) {
          .toc-line {
            animation: none;
            stroke-dasharray: none;
            stroke-dashoffset: 0;
          }
        }
        @keyframes toc-reveal {
          to {
            stroke-dashoffset: 0;
          }
        }
      `}</style>

      <div className="toc-card">
        <div className="toc-header">
          <h3 className="toc-title">Trust grade over time</h3>
          <div className="toc-meta" data-testid="trust-over-time-range">
            {firstLabel} → {lastLabel}
          </div>
        </div>

        <div className="toc-svg-wrap">
          <svg
            className="toc-svg"
            viewBox={`0 0 ${VIEW.width} ${VIEW.height}`}
            role="img"
            aria-label={ariaLabel}
            tabIndex={0}
            onMouseMove={handleMove}
            onMouseLeave={() => setHoverIdx(null)}
            onKeyDown={handleKey}
            onBlur={() => setHoverIdx(null)}
          >
            <defs>
              <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="#8b5cf6" stopOpacity="0.22" />
                <stop offset="100%" stopColor="#8b5cf6" stopOpacity="0" />
              </linearGradient>
            </defs>

            <g className="toc-axis" aria-hidden="true">
              {[0, 1, 2, 3, 4].map((v) => {
                const y = layout.yFor(v);
                return (
                  <g key={v}>
                    <line
                      x1={VIEW.padLeft}
                      x2={VIEW.width - VIEW.padRight}
                      y1={y}
                      y2={y}
                      strokeDasharray="3 3"
                    />
                    <text x={VIEW.padLeft - 6} y={y + 3} textAnchor="end">
                      {GRADE_LABELS[VALUE_GRADE[v]!]}
                    </text>
                  </g>
                );
              })}
              <text
                x={VIEW.padLeft}
                y={VIEW.height - VIEW.padBottom + 18}
                textAnchor="start"
              >
                {firstLabel}
              </text>
              <text
                x={VIEW.width - VIEW.padRight}
                y={VIEW.height - VIEW.padBottom + 18}
                textAnchor="end"
              >
                {lastLabel}
              </text>
            </g>

            <path className="toc-area" d={layout.areaPath} />
            <path
              className="toc-line"
              d={layout.linePath}
              data-testid="trust-over-time-line"
            />

            {hover ? (
              <g aria-hidden="true">
                <line
                  x1={hover.x}
                  x2={hover.x}
                  y1={VIEW.padTop}
                  y2={VIEW.height - VIEW.padBottom}
                  stroke="#c4b5fd"
                  strokeWidth="1"
                  strokeDasharray="2 2"
                />
                <circle className="toc-dot" cx={hover.x} cy={hover.y} r="4.5" />
              </g>
            ) : null}
          </svg>

          {hover ? (
            <div
              className="toc-tooltip"
              data-testid="trust-over-time-tooltip"
              data-grade={hover.point.grade}
              style={{
                left: `${(hover.x / VIEW.width) * 100}%`,
                top: `${(hover.y / VIEW.height) * 100}%`,
              }}
            >
              <div className="toc-tooltip-grade">
                {GRADE_LABELS[hover.point.grade]}
              </div>
              <div>{fmtDate(hover.point.ts)}</div>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

export const __TRUST_CHART_INTERNALS = { gradeOf, fmtDate };
