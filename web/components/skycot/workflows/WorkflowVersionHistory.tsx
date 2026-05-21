"use client";

import { History } from "lucide-react";
import type { WorkflowVersionItem } from "@/lib/api/workflow-rollback";

/**
 * Version-history table for the workflow viewer page. One row per
 * version, sorted newest-first; the row whose `lifecycle_state` is
 * `active` is marked HEAD and never shows a rollback button. Every
 * other row gets a "Rollback to v\<N>" affordance that opens the
 * parent's confirm modal.
 */

export interface WorkflowVersionHistoryProps {
  versions: WorkflowVersionItem[];
  headVersionId: string | null;
  rollbackNotice: string | null;
  onRequestRollback: (target: WorkflowVersionItem) => void;
}

export function WorkflowVersionHistory({
  versions,
  headVersionId,
  rollbackNotice,
  onRequestRollback,
}: WorkflowVersionHistoryProps) {
  if (versions.length === 0) return null;
  const sorted = versions.slice().sort((a, b) => b.version - a.version);
  return (
    <div
      data-testid="wfd-version-history"
      style={{
        background: "white",
        border: "1px solid var(--border, #ece9f5)",
        borderRadius: 12,
        overflow: "hidden",
        fontFamily: "Inter, sans-serif",
        fontSize: 13,
      }}
    >
      {rollbackNotice ? (
        <div
          role="status"
          data-testid="wfd-rollback-toast"
          style={{
            padding: "10px 16px",
            background: "#dcfce7",
            color: "#166534",
            fontSize: 12,
            fontWeight: 500,
          }}
        >
          {rollbackNotice}
        </div>
      ) : null}
      {sorted.map((v) => {
        const isHead = headVersionId !== null && headVersionId === v.id;
        return (
          <div
            key={v.id}
            data-testid={`wfd-version-row-${v.version}`}
            data-is-head={isHead ? "true" : "false"}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 12,
              padding: "10px 16px",
              borderTop: "1px solid var(--border, #ece9f5)",
            }}
          >
            <span style={{ fontWeight: 700, color: "var(--foreground, #1e1b4b)" }}>
              v{v.version}
            </span>
            {isHead ? (
              <span
                style={{
                  padding: "2px 8px",
                  borderRadius: 999,
                  background: "#dcfce7",
                  color: "#166534",
                  fontSize: 10,
                  letterSpacing: "0.14em",
                  textTransform: "uppercase",
                  fontFamily: "JetBrains Mono, monospace",
                }}
              >
                Head
              </span>
            ) : null}
            <span style={{ color: "var(--muted-foreground, #52525b)", fontSize: 12 }}>
              {v.modified_by} · {v.created_at}
            </span>
            <span style={{ flex: 1 }} />
            {!isHead ? (
              <button
                type="button"
                data-testid={`wfd-rollback-button-${v.version}`}
                onClick={() => onRequestRollback(v)}
                className="wfd-btn"
              >
                <History size={13} /> Rollback to v{v.version}
              </button>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}
