"use client";

import type { GrantAccess } from "@/lib/types/capability-grant";

/**
 * Flow-map SVG for the capability-grant card (locked `G2 · Elevated`).
 *
 * Left-to-right illustration of exactly what moves, and which way:
 *
 *   teammate orb  ◂──  connector  ──▸  one node per requested scope
 *
 * Each scope renders one node + one edge. The edge style is keyed on the
 * scope's CURRENT toggle access, so toggling read⇄write live-updates the
 * matching edge:
 *
 *   - read  → teal arrow flowing IN to the teammate (`flow-edge--read`)
 *   - write → purple→pink arrow flowing BOTH ways, animated dash
 *     (`flow-edge--write` + `flow-edge--live`)
 *
 * Pure presentational: the parent owns the scope/access state and passes
 * the resolved `{label, access}[]` rows. Brand tokens come from
 * globals.css (`--brand-primary`, `--brand-pink`, `--brand-teal`); the
 * SVG-gradient stops below are the only literal hexes (SVG `stop-color`
 * can't read CSS custom properties cross-browser) and match the locked
 * tokens exactly.
 */

export interface FlowMapRow {
  id: string;
  label: string;
  access: GrantAccess;
}

export interface CapabilityGrantFlowMapProps {
  teammateName: string;
  connectorName: string;
  rows: FlowMapRow[];
}

// Layout constants. The viewBox grows in height with the scope count so
// nodes never overlap; everything else is fixed.
const NODE_W = 150;
const NODE_H = 44;
const ROW_GAP = 16;
const TOP_PAD = 22;
const ORB_CX = 58;
const CONN_X = 238;
const CONN_W = 122;
const SCOPE_X = 452;
const VIEW_W = 614;

export function CapabilityGrantFlowMap({
  teammateName,
  connectorName,
  rows,
}: CapabilityGrantFlowMapProps) {
  const rowCount = Math.max(rows.length, 1);
  const stackH = rowCount * NODE_H + (rowCount - 1) * ROW_GAP;
  const viewH = stackH + TOP_PAD * 2;
  const midY = viewH / 2;

  const firstName = teammateName.split(" ")[0] || teammateName;
  const initial = (firstName[0] ?? "·").toUpperCase();

  return (
    <svg
      viewBox={`0 0 ${VIEW_W} ${viewH}`}
      role="img"
      aria-label={`Data flow: ${connectorName} scopes flow to ${firstName}`}
      data-testid="capability-grant-flow-map"
    >
      <defs>
        <linearGradient id="cg-node" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#8b5cf6" />
          <stop offset="1" stopColor="#ec4899" />
        </linearGradient>
        <linearGradient id="cg-write" x1="1" y1="0" x2="0" y2="0">
          <stop offset="0" stopColor="#8b5cf6" />
          <stop offset="1" stopColor="#ec4899" />
        </linearGradient>
        <marker
          id="cg-arrow-read"
          viewBox="0 0 10 10"
          refX="7"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          orient="auto-start-reverse"
        >
          <path d="M0 0L10 5L0 10z" fill="#14b8a6" />
        </marker>
        <marker
          id="cg-arrow-write"
          viewBox="0 0 10 10"
          refX="7"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          orient="auto-start-reverse"
        >
          <path d="M0 0L10 5L0 10z" fill="#ec4899" />
        </marker>
      </defs>

      {/* teammate orb */}
      <circle cx={ORB_CX} cy={midY} r="38" fill="url(#cg-node)" />
      <text
        x={ORB_CX}
        y={midY - 4}
        textAnchor="middle"
        fontSize="14"
        fontWeight="700"
        fill="#fff"
      >
        {firstName}
      </text>
      <text
        x={ORB_CX}
        y={midY + 12}
        textAnchor="middle"
        fontFamily="var(--font-mono)"
        fontSize="8"
        fill="#fdeaf5"
      >
        teammate
      </text>

      {/* connector node */}
      <rect
        x={CONN_X}
        y={midY - NODE_H / 2}
        width={CONN_W}
        height={NODE_H}
        rx="13"
        fill="#fff"
        stroke="var(--brand-primary)"
        strokeWidth="1.4"
      />
      <text
        x={CONN_X + CONN_W / 2}
        y={midY + 4}
        textAnchor="middle"
        fontSize="13"
        fontWeight="600"
        fill="var(--foreground)"
      >
        {connectorName}
      </text>

      {/* connector → teammate trunk */}
      <line
        x1={CONN_X}
        y1={midY}
        x2={ORB_CX + 42}
        y2={midY}
        stroke="#bcb4dd"
        strokeWidth="1.6"
        markerEnd="url(#cg-arrow-read)"
        opacity="0.55"
      />

      {/* one node + one edge per scope */}
      {rows.map((row, i) => {
        const nodeY = TOP_PAD + i * (NODE_H + ROW_GAP);
        const nodeMid = nodeY + NODE_H / 2;
        const edgeStartY = midY + (nodeMid < midY ? -10 : nodeMid > midY ? 10 : 0);
        const path = `M${CONN_X + CONN_W} ${edgeStartY} C ${SCOPE_X - 46} ${edgeStartY}, ${SCOPE_X - 38} ${nodeMid}, ${SCOPE_X} ${nodeMid}`;
        const isWrite = row.access === "write";
        return (
          <g key={row.id} data-testid={`flow-scope-${row.id}`}>
            <rect
              x={SCOPE_X}
              y={nodeY}
              width={NODE_W}
              height={NODE_H}
              rx="12"
              fill="#fdfbff"
              stroke={isWrite ? "url(#cg-write)" : "#14b8a6"}
              strokeWidth={isWrite ? "1.7" : "1.5"}
            />
            <circle
              cx={SCOPE_X + 22}
              cy={nodeMid}
              r="11"
              fill={isWrite ? "url(#cg-write)" : "#e7faf6"}
            />
            <text
              x={SCOPE_X + 22}
              y={nodeMid + 4}
              textAnchor="middle"
              fontSize="10"
              fontWeight="700"
              fill={isWrite ? "#fff" : "#0f766e"}
            >
              {isWrite ? "W" : "R"}
            </text>
            <text
              x={SCOPE_X + 40}
              y={nodeMid - 2}
              fontFamily="var(--font-mono)"
              fontSize="11"
              fontWeight="600"
              fill="var(--foreground)"
            >
              {row.label}
            </text>
            <text
              x={SCOPE_X + 40}
              y={nodeMid + 12}
              fontSize="9.5"
              fill={isWrite ? "#be185d" : "#0f766e"}
            >
              {isWrite ? "read · write" : "read"}
            </text>
            <path
              data-testid={`flow-edge-${row.id}`}
              data-access={row.access}
              className={
                isWrite
                  ? "cg-flow-edge cg-flow-edge--write cg-flow-edge--live"
                  : "cg-flow-edge cg-flow-edge--read"
              }
              d={path}
              fill="none"
              stroke={isWrite ? "url(#cg-write)" : "#14b8a6"}
              strokeWidth={isWrite ? "3.4" : "2.4"}
              markerStart={`url(#cg-arrow-${row.access})`}
              markerEnd={isWrite ? "url(#cg-arrow-write)" : undefined}
            />
          </g>
        );
      })}
    </svg>
  );
}
