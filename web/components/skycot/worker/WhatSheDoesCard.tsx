"use client";

import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Pencil } from "lucide-react";
import { Card, CardBody, CardHeader, CardTitle } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

/**
 * "What she does" card — renders the worker charter (the 3-sentence
 * description of intent + boundaries the worker is hired against) and
 * supports inline edit when `canEdit` is true.
 *
 * Save on blur or Enter (Shift+Enter inserts newline); Escape
 * reverts. The actual persistence pathway (charter PATCH endpoint +
 * audit log) lands with E4-g — this v1 surface accepts an
 * `onSave(next)` callback so the parent page can wire it as soon as
 * the mutation hook exists.
 *
 * "✎ Adjust" opens the RetrainDialog from E4-g; v1 stubs that as a
 * toast via `onAdjust()`.
 */

export interface WhatSheDoesCardProps {
  charter: string;
  canEdit: boolean;
  /** Called when the user commits an edit (blur / Enter). */
  onSave?: (next: string) => void;
  /** Called when the "Adjust" button is clicked. */
  onAdjust?: () => void;
  /** Shows the per-card error state instead of the charter body. */
  error?: string | null;
  /** Retry handler for the error state. */
  onRetry?: () => void;
  className?: string;
}

export function WhatSheDoesCard({
  charter,
  canEdit,
  onSave,
  onAdjust,
  error,
  onRetry,
  className,
}: WhatSheDoesCardProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(charter);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    if (!editing) setDraft(charter);
  }, [charter, editing]);

  useEffect(() => {
    if (editing && textareaRef.current) {
      textareaRef.current.focus();
      const len = textareaRef.current.value.length;
      textareaRef.current.setSelectionRange(len, len);
    }
  }, [editing]);

  const commit = () => {
    const next = draft.trim();
    setEditing(false);
    if (next && next !== charter) onSave?.(next);
    else setDraft(charter);
  };

  const cancel = () => {
    setDraft(charter);
    setEditing(false);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Escape") {
      e.preventDefault();
      cancel();
    } else if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      commit();
    }
  };

  return (
    <Card data-testid="card-what-she-does" className={className}>
      <CardHeader className="flex flex-row items-start justify-between gap-2">
        <CardTitle>What she does</CardTitle>
        <Button
          variant="ghost"
          size="sm"
          onClick={onAdjust}
          data-testid="charter-adjust"
          aria-label="Adjust how this worker behaves"
        >
          <Pencil className="size-3.5" />
          Adjust
        </Button>
      </CardHeader>
      <CardBody className="pt-1">
        {error ? (
          <CardErrorState message={error} onRetry={onRetry} />
        ) : editing ? (
          <textarea
            ref={textareaRef}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={onKeyDown}
            rows={4}
            data-testid="charter-editor"
            aria-label="Edit charter"
            className={cn(
              "w-full resize-none rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)]",
              "px-3 py-2 text-sm leading-relaxed text-[var(--foreground)]",
              "focus:outline-none focus:ring-2 focus:ring-[var(--ring)]",
            )}
          />
        ) : (
          <p
            data-testid="charter-text"
            onClick={canEdit ? () => setEditing(true) : undefined}
            role={canEdit ? "button" : undefined}
            tabIndex={canEdit ? 0 : undefined}
            onKeyDown={
              canEdit
                ? (e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      setEditing(true);
                    }
                  }
                : undefined
            }
            className={cn(
              "text-sm leading-relaxed text-[var(--foreground)]/90 whitespace-pre-wrap",
              canEdit &&
                "cursor-text rounded-[var(--radius-sm)] -mx-1 px-1 hover:bg-[var(--accent)]/40 focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]",
            )}
          >
            {charter}
          </p>
        )}
      </CardBody>
    </Card>
  );
}

function CardErrorState({
  message,
  onRetry,
}: {
  message: string;
  onRetry?: () => void;
}) {
  return (
    <div className="space-y-2 text-sm text-[var(--muted-foreground)]">
      <p>{message}</p>
      {onRetry ? (
        <button
          type="button"
          onClick={onRetry}
          className="text-[var(--primary)] underline-offset-4 hover:underline"
        >
          Retry
        </button>
      ) : null}
    </div>
  );
}

/** Skeleton rendered while the overview is loading. */
export function WhatSheDoesCardSkeleton() {
  return (
    <Card data-testid="card-what-she-does-skeleton">
      <CardHeader>
        <div className="h-4 w-32 rounded bg-[var(--muted)]" />
      </CardHeader>
      <CardBody className="pt-1 space-y-2">
        <div className="h-3 w-full rounded bg-[var(--muted)]" />
        <div className="h-3 w-11/12 rounded bg-[var(--muted)]" />
        <div className="h-3 w-9/12 rounded bg-[var(--muted)]" />
      </CardBody>
    </Card>
  );
}
