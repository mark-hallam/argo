"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { History, X } from "lucide-react";
import { useState } from "react";
import { cn } from "@/lib/utils";

/**
 * Rollback confirmation modal — shows the diff between the current
 * head and the target version, asks the operator to confirm, then
 * delegates to `onConfirm` (which POSTs to the rollback endpoint and
 * refreshes the viewer). Built on Radix's Dialog primitive so focus
 * trap, Escape close, and aria wiring come for free.
 */

export interface RollbackConfirmModalProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  currentVersion: number;
  targetVersion: number;
  currentToml: string;
  targetToml: string;
}

export function RollbackConfirmModal({
  open,
  onClose,
  onConfirm,
  currentVersion,
  targetVersion,
  currentToml,
  targetToml,
}: RollbackConfirmModalProps) {
  const [submitting, setSubmitting] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const handleConfirm = async () => {
    setErrorMessage(null);
    setSubmitting(true);
    try {
      await onConfirm();
    } catch (e) {
      setErrorMessage(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  };

  const handleOpenChange = (next: boolean) => {
    if (!next && !submitting) {
      setErrorMessage(null);
      onClose();
    }
  };

  return (
    <DialogPrimitive.Root open={open} onOpenChange={handleOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay
          className={cn(
            "fixed inset-0 z-40 bg-black/40 backdrop-blur-sm",
            "data-[state=open]:animate-in data-[state=closed]:animate-out",
          )}
        />
        <DialogPrimitive.Content
          aria-modal="true"
          data-testid="rollback-confirm-modal"
          className={cn(
            "fixed inset-0 z-50 bg-[var(--background)] sm:inset-auto sm:left-1/2 sm:top-1/2",
            "sm:w-[92vw] sm:max-w-3xl sm:-translate-x-1/2 sm:-translate-y-1/2",
            "sm:rounded-[var(--radius-lg)] sm:border sm:border-[var(--border)] sm:shadow-lg",
            "flex flex-col sm:max-h-[85vh] focus:outline-none",
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
                  data-testid="rollback-confirm-title"
                >
                  Rollback to v{targetVersion}
                </DialogPrimitive.Title>
                <DialogPrimitive.Description className="mt-0.5 text-xs text-[var(--muted-foreground)]">
                  Creates a new version copied from v{targetVersion} and promotes it to head. History is not rewritten.
                </DialogPrimitive.Description>
              </div>
            </div>
            <DialogPrimitive.Close
              aria-label="Close"
              data-testid="rollback-confirm-close"
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

          <div className="grid grid-cols-1 gap-4 px-6 py-4 sm:grid-cols-2">
            <section>
              <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">
                Current — v{currentVersion}
              </h3>
              <pre
                data-testid="rollback-diff-current"
                className="max-h-[40vh] overflow-auto rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)] p-3 font-mono text-xs text-[var(--foreground)]"
              >
                {currentToml}
              </pre>
            </section>
            <section>
              <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-[var(--muted-foreground)]">
                Target — v{targetVersion}
              </h3>
              <pre
                data-testid="rollback-diff-target"
                className="max-h-[40vh] overflow-auto rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--muted)] p-3 font-mono text-xs text-[var(--foreground)]"
              >
                {targetToml}
              </pre>
            </section>
          </div>

          {errorMessage ? (
            <div
              role="alert"
              data-testid="rollback-confirm-error"
              className="mx-6 mb-2 rounded-[var(--radius-md)] border border-red-300 bg-red-50 px-3 py-2 text-xs text-red-700"
            >
              {errorMessage}
            </div>
          ) : null}

          <footer className="flex items-center justify-end gap-2 border-t border-[var(--border)] px-6 py-4">
            <button
              type="button"
              data-testid="rollback-confirm-cancel"
              onClick={() => handleOpenChange(false)}
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
              data-testid="rollback-confirm-submit"
              onClick={handleConfirm}
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
  );
}
