"use client";

/**
 * Run-history stat grid + recent-runs table for the
 * `/workflows/<v>/<n>` detail page.
 *
 * Extracted from `page.tsx` to keep that file under the 500-LOC
 * ceiling (the brief's swap-in pushed page.tsx past the limit).
 *
 * Backed by `GET /api/v1/verticals/<v>/workflows/<n>/runs` — replaces
 * the hardcoded `82` runs / `94%` literals.
 */

import type { WorkflowRun } from "@/lib/api/workflow-runs";

interface WorkflowRunStatsProps {
  query: { isPending: boolean; isError: boolean };
  summary:
    | {
        run_count: number;
        success_rate: number | null;
        avg_duration_ms: number;
        last_run_at: string | null;
      }
    | null;
}

export function WorkflowRunStats({ query, summary }: WorkflowRunStatsProps) {
  const isPending = query.isPending;
  const isEmpty =
    !isPending && !query.isError && (summary?.run_count ?? 0) === 0;
  return (
    <>
      <div
        data-testid="wfd-run-stats"
        style={{
          padding: 20,
          background: "white",
          border: "1px solid var(--border, #ece9f5)",
          borderRadius: 12,
          display: "flex",
          gap: 24,
          flexWrap: "wrap",
          fontFamily: "Inter, sans-serif",
          fontSize: 13,
          color: "var(--muted-foreground, #52525b)",
        }}
      >
        <StatCell
          label="Runs"
          value={
            isPending || query.isError
              ? "—"
              : String(summary?.run_count ?? 0)
          }
        />
        <StatCell
          label="Success rate"
          color={
            !isPending && !query.isError && summary?.success_rate != null
              ? "#14b8a6"
              : undefined
          }
          value={
            isPending || query.isError || summary?.success_rate == null
              ? "—"
              : `${Math.round(summary.success_rate * 100)}%`
          }
        />
        <StatCell
          label="Avg duration"
          value={
            isPending || query.isError || !summary
              ? "—"
              : formatDuration(summary.avg_duration_ms)
          }
        />
        <StatCell
          label="Last run"
          value={
            isPending || query.isError
              ? "—"
              : summary?.last_run_at
                ? relativeTime(summary.last_run_at)
                : "Never"
          }
        />
      </div>
      {isEmpty ? (
        <div
          data-testid="wfd-runs-empty"
          style={{
            marginTop: 12,
            padding: "14px 16px",
            border: "1px dashed var(--border, #ece9f5)",
            borderRadius: 10,
            color: "var(--muted-foreground, #52525b)",
            fontSize: 13,
            lineHeight: 1.5,
          }}
        >
          This workflow hasn&rsquo;t been run yet. Open the builder and hit
          <strong> Test run</strong> to see your first stats here.
        </div>
      ) : null}
    </>
  );
}

function StatCell({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color?: string;
}) {
  return (
    <div>
      <div
        data-testid={`wfd-stat-${label.toLowerCase().replace(/\s+/g, "-")}`}
        style={{
          fontSize: 28,
          fontWeight: 900,
          color: color ?? "var(--foreground, #1e1b4b)",
          lineHeight: 1,
        }}
      >
        {value}
      </div>
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          letterSpacing: "0.14em",
          textTransform: "uppercase",
          marginTop: 4,
        }}
      >
        {label}
      </div>
    </div>
  );
}

interface WorkflowRunsTableProps {
  isPending: boolean;
  isError: boolean;
  runs: WorkflowRun[];
}

export function WorkflowRunsTable({
  isPending,
  isError,
  runs,
}: WorkflowRunsTableProps) {
  if (isPending || isError || runs.length === 0) {
    return null;
  }
  return (
    <>
      <div className="wfd-section-label">RECENT RUNS</div>
      <div
        data-testid="wfd-runs-table"
        style={{
          background: "white",
          border: "1px solid var(--border, #ece9f5)",
          borderRadius: 12,
          overflow: "hidden",
        }}
      >
        <table
          style={{
            width: "100%",
            borderCollapse: "collapse",
            fontFamily: "Inter, sans-serif",
            fontSize: 13,
          }}
        >
          <thead>
            <tr
              style={{
                background: "rgba(139, 92, 246, 0.04)",
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                color: "var(--muted-foreground, #52525b)",
              }}
            >
              <th style={{ textAlign: "left", padding: "10px 16px" }}>Run ID</th>
              <th style={{ textAlign: "left", padding: "10px 16px" }}>Started</th>
              <th style={{ textAlign: "left", padding: "10px 16px" }}>Duration</th>
              <th style={{ textAlign: "left", padding: "10px 16px" }}>Status</th>
            </tr>
          </thead>
          <tbody>
            {runs.map((r) => (
              <tr
                key={r.id}
                style={{
                  borderTop: "1px solid var(--border, #ece9f5)",
                  color: "var(--foreground, #1e1b4b)",
                }}
              >
                <td
                  style={{
                    padding: "12px 16px",
                    fontFamily: "var(--font-mono)",
                    fontSize: 12,
                  }}
                >
                  <code>{shortHash(r.id)}</code>
                </td>
                <td
                  style={{
                    padding: "12px 16px",
                    color: "var(--muted-foreground, #52525b)",
                  }}
                >
                  {relativeTime(r.started_at)}
                </td>
                <td
                  style={{
                    padding: "12px 16px",
                    color: "var(--muted-foreground, #52525b)",
                  }}
                >
                  {r.duration_ms != null
                    ? formatDuration(r.duration_ms)
                    : "Running…"}
                </td>
                <td style={{ padding: "12px 16px" }}>
                  <span
                    style={{
                      display: "inline-block",
                      padding: "2px 9px",
                      borderRadius: 999,
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      letterSpacing: "0.14em",
                      textTransform: "uppercase",
                      fontWeight: 700,
                      background: r.ok ? "#dcfce7" : "#fee2e2",
                      color: r.ok ? "#166534" : "#991b1b",
                    }}
                  >
                    {r.ok ? "Succeeded" : "Failed"}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

function shortHash(id: string): string {
  if (id.length <= 10) return id;
  return `${id.slice(0, 8)}…`;
}

export function formatDuration(ms: number): string {
  if (ms < 1_000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1_000).toFixed(1)}s`;
  const minutes = Math.floor(ms / 60_000);
  const seconds = Math.round((ms % 60_000) / 1_000);
  return `${minutes}m ${seconds}s`;
}

export function relativeTime(iso: string): string {
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return iso;
  const deltaSec = (Date.now() - then) / 1_000;
  if (deltaSec < 60) return "just now";
  if (deltaSec < 3_600) return `${Math.floor(deltaSec / 60)}m ago`;
  if (deltaSec < 86_400) return `${Math.floor(deltaSec / 3_600)}h ago`;
  if (deltaSec < 604_800) return `${Math.floor(deltaSec / 86_400)}d ago`;
  return new Date(then).toISOString().slice(0, 10);
}
