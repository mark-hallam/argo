"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";

/**
 * Add-a-fact dialog. Modal with a free-text textarea (≥ 4 rows)
 * plus Save / Cancel. Hands the trimmed text back through
 * `onSave(text)`; the parent decides whether to commit
 * synchronously (mock) or async (gateway POST).
 *
 * Reuses `@radix-ui/react-dialog` directly — same primitive that
 * the workspace's `AlertDialog` wraps — so we stay inside the
 * already-installed dependency set.
 */

export interface AddFactDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Worker first-name; surfaces in the heading. */
  workerName: string;
  /** Returns a promise so the dialog can hold the button in a
   *  pending state while the optimistic mutation resolves. */
  onSave: (text: string) => void | Promise<void>;
}

export function AddFactDialog({
  open,
  onOpenChange,
  workerName,
  onSave,
}: AddFactDialogProps) {
  const [draft, setDraft] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    if (!open) {
      setDraft("");
      setSubmitting(false);
    }
  }, [open]);

  useEffect(() => {
    if (open && textareaRef.current) {
      textareaRef.current.focus();
    }
  }, [open]);

  const handleSave = async () => {
    const text = draft.trim();
    if (!text || submitting) return;
    try {
      setSubmitting(true);
      await onSave(text);
      onOpenChange(false);
    } finally {
      setSubmitting(false);
    }
  };

  const disabled = draft.trim().length === 0 || submitting;

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          className={cn(
            "fixed inset-0 z-40 bg-black/40 backdrop-blur-sm",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
          )}
        />
        <DialogPrimitive.Content
          aria-modal="true"
          className={cn(
            "fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg",
            "focus:outline-none",
          )}
          data-testid="add-fact-dialog"
        >
          <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
            Teach {workerName} a new fact
          </DialogPrimitive.Title>
          <DialogPrimitive.Description className="mt-1 text-sm text-[var(--muted-foreground)]">
            Plain English. She&apos;ll apply it next time the situation comes up.
          </DialogPrimitive.Description>

          <textarea
            ref={textareaRef}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            rows={5}
            data-testid="add-fact-textarea"
            aria-label="New fact text"
            placeholder="e.g. Refunds under $50 auto-approve. Above $50 ask Marta."
            className={cn(
              "mt-4 w-full resize-none rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)]",
              "px-3 py-2 text-sm leading-relaxed text-[var(--foreground)]",
              "focus:outline-none focus:ring-2 focus:ring-[var(--ring)]",
            )}
          />

          <div className="mt-6 flex justify-end gap-2">
            <Button
              variant="ghost"
              data-testid="add-fact-cancel"
              onClick={() => onOpenChange(false)}
              disabled={submitting}
            >
              Cancel
            </Button>
            <Button
              variant="primary"
              data-testid="add-fact-save"
              onClick={handleSave}
              disabled={disabled}
              loading={submitting}
            >
              Save
            </Button>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
