"use client";

import { useState } from "react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { useRecentRetrains, useUndoRetrain } from "@/lib/hooks/use-retrain";
import type { RetrainRecord } from "@/lib/api/retrain";
import { cn } from "@/lib/utils";

/**
 * RecentChangesPanel — subtle entry on the Overview tab's bottom
 * row that surfaces the worker's last 14 days of retrains, newest
 * first. Each row carries a thin one-line summary and an [Undo]
 * button while inside the 14-day TTL.
 *
 * Source of truth: `useRecentRetrains` (TanStack Query). v1 falls
 * back to an empty list when the gateway endpoint isn't wired;
 * commit mutations from RetrainDialog optimistically prepend new
 * records so the panel surfaces them immediately.
 */

export interface RecentChangesPanelProps {
  workerId: string;
}

function relativeTime(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  if (ms < 60_000) return "just now";
  const minutes = Math.floor(ms / 60_000);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  const weeks = Math.floor(days / 7);
  if (weeks < 4) return `${weeks}w ago`;
  return new Date(iso).toLocaleDateString();
}

function isWithinUndoWindow(record: RetrainRecord): boolean {
  if (record.undoneAt) return false;
  return new Date(record.undoExpiresAt).getTime() > Date.now();
}

export function RecentChangesPanel({ workerId }: RecentChangesPanelProps) {
  const query = useRecentRetrains(workerId);
  const undoMutation = useUndoRetrain(workerId);
  const [confirmTarget, setConfirmTarget] = useState<RetrainRecord | null>(null);

  if (query.isPending) {
    return (
      <Card data-testid="card-recent-changes-skeleton">
        <CardHeader>
          <div className="h-4 w-32 rounded bg-[var(--muted)]" />
        </CardHeader>
        <CardBody className="pt-1 space-y-2">
          <div className="h-3 w-full rounded bg-[var(--muted)]" />
          <div className="h-3 w-9/12 rounded bg-[var(--muted)]" />
        </CardBody>
      </Card>
    );
  }

  const records = query.data?.records ?? [];

  if (records.length === 0) {
    return (
      <Card data-testid="card-recent-changes">
        <CardHeader>
          <CardTitle>Recent changes</CardTitle>
        </CardHeader>
        <CardBody className="pt-1">
          <p
            data-testid="recent-changes-empty"
            className="text-sm text-[var(--muted-foreground)]"
          >
            No retrains yet. Click <em>Adjust</em> on any card above to teach
            her something new.
          </p>
        </CardBody>
      </Card>
    );
  }

  return (
    <Card data-testid="card-recent-changes">
      <CardHeader>
        <CardTitle>Recent changes</CardTitle>
      </CardHeader>
      <CardBody className="pt-1">
        <ul role="list" className="divide-y divide-[var(--border)]/60">
          {records.map((r) => {
            const canUndo = isWithinUndoWindow(r);
            return (
              <li
                key={r.retrainId}
                data-testid={`recent-changes-row-${r.retrainId}`}
                data-undone={r.undoneAt ? "true" : "false"}
                className="flex items-center justify-between gap-3 py-2 text-sm"
              >
                <div className="min-w-0 flex-1">
                  <p
                    className={cn(
                      "truncate text-[var(--foreground)]",
                      r.undoneAt &&
                        "text-[var(--muted-foreground)] line-through",
                    )}
                  >
                    <span className="mr-2 text-xs text-[var(--muted-foreground)]">
                      {relativeTime(r.committedAt)}
                    </span>
                    {r.oneLineSummary}
                  </p>
                  {r.undoneAt ? (
                    <p className="text-xs text-[var(--muted-foreground)]">
                      undone {relativeTime(r.undoneAt)}
                    </p>
                  ) : null}
                </div>
                {canUndo ? (
                  <Button
                    variant="ghost"
                    size="sm"
                    data-testid={`recent-changes-undo-${r.retrainId}`}
                    onClick={() => setConfirmTarget(r)}
                    disabled={undoMutation.isPending}
                  >
                    Undo
                  </Button>
                ) : null}
              </li>
            );
          })}
        </ul>
      </CardBody>

      <AlertDialog
        open={confirmTarget !== null}
        onOpenChange={(open) => {
          if (!open) setConfirmTarget(null);
        }}
        title="Undo this change?"
        description={
          confirmTarget
            ? `She'll go back to how she was before this rule. (${confirmTarget.oneLineSummary})`
            : undefined
        }
        confirmLabel="Undo"
        cancelLabel="Keep it"
        confirmVariant="destructive"
        onConfirm={() => {
          if (confirmTarget) {
            undoMutation.mutate({ retrainId: confirmTarget.retrainId });
            setConfirmTarget(null);
          }
        }}
      />
    </Card>
  );
}
