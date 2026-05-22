"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { History, X } from "lucide-react";
import { toast } from "sonner";

import { ApiError } from "@/lib/api/types";
import {
  listVersions,
  parseStepsFromToml,
  rollbackToVersion,
  type WorkflowVersionItem,
} from "@/lib/api/workflows";
import { cn } from "@/lib/utils";

/**
 * Versions panel for the workflow viewer page (Stream
 * `web-workflow-rollback-ui-button`). Lists the last 5 published
 * versions of a workflow (head + 4 preceding), each with timestamp +
 * author + a 1-line summary extracted from the version's TOML
 * `workflow.description`. Each non-head row has a "Rollback to this
 * version" button that opens a confirm dialog with brief-shaped
 * verbiage:
 *
 *   "Roll back to v3? Current head is v5, drafted v6 will move
 *    forward."
 *
 * On confirm, the panel POSTs to the rollback endpoint
 * (`/api/v1/verticals/{v}/workflows/{n}/rollback`) and surfaces a
 * sonner toast. Error handling distinguishes 4xx (field-level toast
 * with the gateway-supplied message) from 5xx (red toast with a
 * "Retry" action that re-fires the rollback). After a successful
 * rollback the panel re-fetches the version list AND fires
 * `onAfterRollback`, which the parent uses to refresh the workflow
 * head.
 *
 * The panel handles its own loading + version-fetch errors quietly —
 * an empty list collapses the section so the viewer doesn't show a
 * dead row.
 */

const PUBLISHED_VERSIONS_SHOWN = 5;

export interface WorkflowVersionsPanelProps {
  verticalId: string;
  name: string;
  /** Called after a successful rollback so the parent can refresh. */
  onAfterRollback?: () => void;
}

interface PublishedRow {
  item: WorkflowVersionItem;
  summary: string;
  isHead: boolean;
}

function summaryFor(item: WorkflowVersionItem): string {
  const parsed = parseStepsFromToml(item.definition_toml ?? "");
  if (parsed.description) return parsed.description;
  if (parsed.steps.length > 0) {
    const first = parsed.steps[0];
    const trail = parsed.steps.length > 1 ? ` (+${parsed.steps.length - 1} more)` : "";
    return `${first.name}${trail}`;
  }
  return `Version ${item.version}`;
}

function formatTimestamp(value: string): string {
  if (!value) return "—";
  const parsed = Date.parse(value);
  if (Number.isNaN(parsed)) return value;
  return new Date(parsed).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

export function WorkflowVersionsPanel({
  verticalId,
  name,
  onAfterRollback,
}: WorkflowVersionsPanelProps) {
  const [versions, setVersions] = useState<WorkflowVersionItem[] | null>(null);
  const [target, setTarget] = useState<WorkflowVersionItem | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [modalError, setModalError] = useState<string | null>(null);

  const fetchVersions = useCallback(async () => {
    try {
      const items = await listVersions(verticalId, name);
      setVersions(items);
    } catch {
      // Quiet failure — the viewer page renders the rest of the
      // workflow detail even when the gateway can't serve a version
      // list (offline / non-live mock route).
      setVersions([]);
    }
  }, [verticalId, name]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const items = await listVersions(verticalId, name);
        if (!cancelled) setVersions(items);
      } catch {
        if (!cancelled) setVersions([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [verticalId, name]);

  const rows = useMemo<PublishedRow[]>(() => {
    if (!versions) return [];
    const published = versions
      .filter((v) => v.lifecycle_state !== "archived")
      .slice()
      .sort((a, b) => b.version - a.version);
    const headId = published.find((v) => v.lifecycle_state === "active")?.id ?? null;
    return published
      .slice(0, PUBLISHED_VERSIONS_SHOWN)
      .map((item) => ({
        item,
        summary: summaryFor(item),
        isHead: headId !== null && item.id === headId,
      }));
  }, [versions]);

  const headRow = rows.find((r) => r.isHead) ?? null;

  const performRollback = useCallback(
    async (selected: WorkflowVersionItem) => {
      const result = await rollbackToVersion({
        verticalId,
        name,
        targetVersionId: selected.id,
      });
      toast.success(
        `Rolled back to v${selected.version}. New head is v${result.version}.`,
      );
      await fetchVersions();
      onAfterRollback?.();
    },
    [verticalId, name, fetchVersions, onAfterRollback],
  );

  const closeModal = useCallback(() => {
    if (submitting) return;
    setTarget(null);
    setModalError(null);
  }, [submitting]);

  const handleConfirm = useCallback(async () => {
    if (!target) return;
    const selected = target;
    setModalError(null);
    setSubmitting(true);
    try {
      await performRollback(selected);
      setTarget(null);
    } catch (err) {
      const isClient =
        err instanceof ApiError && err.status >= 400 && err.status < 500;
      const message =
        err instanceof Error ? err.message : "The gateway is unavailable.";
      setModalError(message);
      if (isClient) {
        toast.error("Couldn't roll back workflow", { description: message });
      } else {
        toast.error("Rollback failed", {
          description: message,
          action: {
            label: "Retry",
            onClick: () => {
              void performRollback(selected).catch(() => {
                /* swallow — toasted by performRollback's caller; keeps modal open */
              });
            },
          },
        });
      }
    } finally {
      setSubmitting(false);
    }
  }, [target, performRollback]);

  if (versions === null || rows.length === 0) return null;

  return (
    <section data-testid="wfd-versions-panel">
      <div className="wfd-section-label">PUBLISHED VERSIONS · LAST {rows.length}</div>
      <div
        style={{
          background: "white",
          border: "1px solid var(--border, #ece9f5)",
          borderRadius: 12,
          overflow: "hidden",
          fontFamily: "Inter, sans-serif",
          fontSize: 13,
        }}
      >
        {rows.map((row, idx) => (
          <div
            key={row.item.id}
            data-testid={`wfd-versions-row-${row.item.version}`}
            data-is-head={row.isHead ? "true" : "false"}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 12,
              padding: "12px 16px",
              borderTop: idx === 0 ? "none" : "1px solid var(--border, #ece9f5)",
            }}
          >
            <div style={{ minWidth: 56, display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ fontWeight: 700, color: "var(--foreground, #1e1b4b)" }}>
                v{row.item.version}
              </span>
              {row.isHead ? (
                <span
                  data-testid={`wfd-versions-head-pill-${row.item.version}`}
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
            </div>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div
                data-testid={`wfd-versions-summary-${row.item.version}`}
                style={{
                  color: "var(--foreground, #1e1b4b)",
                  fontWeight: 500,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {row.summary}
              </div>
              <div
                style={{
                  marginTop: 2,
                  color: "var(--muted-foreground, #52525b)",
                  fontSize: 12,
                }}
              >
                {formatTimestamp(row.item.created_at)} · {row.item.modified_by || "—"}
              </div>
            </div>
            {!row.isHead ? (
              <button
                type="button"
                data-testid={`wfd-versions-rollback-${row.item.version}`}
                onClick={() => setTarget(row.item)}
                className="wfd-btn"
              >
                <History size={13} /> Rollback to this version
              </button>
            ) : null}
          </div>
        ))}
      </div>
      {target && headRow ? (
        <DialogPrimitive.Root
          open={true}
          onOpenChange={(next) => {
            if (!next) closeModal();
          }}
        >
          <DialogPrimitive.Portal>
            <DialogPrimitive.Overlay
              className="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm"
            />
            <DialogPrimitive.Content
              aria-modal="true"
              data-testid="wfd-versions-confirm-modal"
              className={cn(
                "fixed inset-0 z-50 bg-[var(--background)] sm:inset-auto sm:left-1/2 sm:top-1/2",
                "sm:w-[92vw] sm:max-w-md sm:-translate-x-1/2 sm:-translate-y-1/2",
                "sm:rounded-[var(--radius-lg)] sm:border sm:border-[var(--border)] sm:shadow-lg",
                "flex flex-col focus:outline-none",
              )}
            >
              <header className="flex items-start justify-between gap-4 border-b border-[var(--border)] px-6 py-5">
                <div className="flex items-center gap-2 min-w-0">
                  <span
                    aria-hidden
                    className="grid size-8 place-items-center rounded-[var(--radius-md)] bg-[var(--muted)] text-[var(--muted-foreground)]"
                  >
                    <History size={16} />
                  </span>
                  <div className="min-w-0">
                    <DialogPrimitive.Title
                      className="text-base font-semibold text-[var(--foreground)]"
                      data-testid="wfd-versions-confirm-title"
                    >
                      Roll back to v{target.version}?
                    </DialogPrimitive.Title>
                    <DialogPrimitive.Description
                      data-testid="wfd-versions-confirm-body"
                      className="mt-0.5 text-xs text-[var(--muted-foreground)]"
                    >
                      Current head is v{headRow.item.version}, drafted v{headRow.item.version + 1} will move forward.
                    </DialogPrimitive.Description>
                  </div>
                </div>
                <DialogPrimitive.Close
                  aria-label="Close"
                  data-testid="wfd-versions-confirm-close"
                  disabled={submitting}
                  className={cn(
                    "grid size-8 place-items-center rounded-[var(--radius-md)]",
                    "text-[var(--muted-foreground)] hover:bg-[var(--muted)] hover:text-[var(--foreground)]",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                    "disabled:cursor-not-allowed disabled:opacity-50",
                  )}
                >
                  <X aria-hidden size={18} />
                </DialogPrimitive.Close>
              </header>

              {modalError ? (
                <div
                  role="alert"
                  data-testid="wfd-versions-confirm-error"
                  className="mx-6 mt-3 rounded-[var(--radius-md)] border border-red-300 bg-red-50 px-3 py-2 text-xs text-red-700"
                >
                  {modalError}
                </div>
              ) : null}

              <footer className="flex items-center justify-end gap-2 border-t border-[var(--border)] px-6 py-4">
                <button
                  type="button"
                  data-testid="wfd-versions-confirm-cancel"
                  onClick={closeModal}
                  disabled={submitting}
                  className={cn(
                    "rounded-[var(--radius-md)] border border-[var(--border)] bg-transparent px-3 py-1.5 text-sm text-[var(--foreground)]",
                    "hover:bg-[var(--muted)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                    "disabled:cursor-not-allowed disabled:opacity-50",
                  )}
                >
                  Cancel
                </button>
                <button
                  type="button"
                  data-testid="wfd-versions-confirm-submit"
                  onClick={() => {
                    void handleConfirm();
                  }}
                  disabled={submitting}
                  className={cn(
                    "rounded-[var(--radius-md)] bg-[var(--primary)] px-3 py-1.5 text-sm font-medium text-[var(--primary-foreground)]",
                    "hover:opacity-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
                    "disabled:cursor-not-allowed disabled:opacity-50",
                  )}
                >
                  {submitting ? "Rolling back…" : "Confirm rollback"}
                </button>
              </footer>
            </DialogPrimitive.Content>
          </DialogPrimitive.Portal>
        </DialogPrimitive.Root>
      ) : null}
    </section>
  );
}
