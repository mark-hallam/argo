"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useEffect, useMemo, useState } from "react";
import { Loader2, ShieldQuestion, X } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import { useElevatedViewGrants, GRANT_DURATION_LABELS, type GrantDuration } from "@/lib/hooks/use-elevated-view-grant";
import { REQUESTABLE_CATEGORIES } from "@/lib/redaction/audit-rules";
import { submitElevatedViewRequest } from "@/lib/api/elevated-view";
import type { RbacPermission } from "@/lib/redaction/types";

/**
 * Modal that captures an elevated-view request: reason (≥20 chars),
 * which sentinels to unlock, and an expiry duration. On submit it
 * POSTs the audit-the-audit row to the gateway (mock-falls-through
 * in v1) and issues a client-side grant via `useElevatedViewGrants`.
 *
 * The dialog is fully controlled by the caller — surface-level state
 * (banner clicks, sentinel clicks) owns the open/close lifecycle so
 * a single dialog can serve multiple triggers.
 */

const MIN_REASON_LEN = 20;
const DURATION_OPTIONS: readonly GrantDuration[] = ["1h", "4h", "8h", "24h"] as const;

export interface ElevatedViewRequestDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Categories pre-selected when the dialog opens (sentinel click). */
  initialCategories?: RbacPermission[];
  /** Surface identifier — e.g. `"audit-feed"` or `"audit-detail"`. */
  surface: string;
  /** Optional callback fired after a successful grant — closes any
   *  upstream "Some fields masked" UI. */
  onGranted?: (granted: RbacPermission[]) => void;
}

export function ElevatedViewRequestDialog({
  open,
  onOpenChange,
  initialCategories,
  surface,
  onGranted,
}: ElevatedViewRequestDialogProps) {
  const { issueGrant } = useElevatedViewGrants();
  const [reason, setReason] = useState("");
  const [duration, setDuration] = useState<GrantDuration>("1h");
  const [selected, setSelected] = useState<Set<RbacPermission>>(() => new Set(initialCategories ?? []));
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setReason("");
    setDuration("1h");
    setSelected(new Set(initialCategories ?? []));
    setError(null);
    setSubmitting(false);
  }, [open, initialCategories]);

  const reasonTooShort = reason.trim().length < MIN_REASON_LEN;
  const noCategories = selected.size === 0;
  const submitDisabled = submitting || reasonTooShort || noCategories;
  const reasonHint = useMemo(() => {
    if (reasonTooShort) {
      return `${MIN_REASON_LEN - reason.trim().length} more character${
        MIN_REASON_LEN - reason.trim().length === 1 ? "" : "s"
      } required.`;
    }
    return null;
  }, [reason, reasonTooShort]);

  function toggleCategory(perm: RbacPermission) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(perm)) next.delete(perm);
      else next.add(perm);
      return next;
    });
  }

  async function handleSubmit() {
    if (submitDisabled) return;
    setSubmitting(true);
    setError(null);
    const categories = Array.from(selected);
    try {
      await submitElevatedViewRequest({ surface, categories, reason: reason.trim(), duration });
      issueGrant({ categories, reason: reason.trim(), duration });
      onGranted?.(categories);
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Couldn't submit request");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm" />
        <DialogPrimitive.Content
          aria-modal="true"
          data-testid="elevated-view-dialog"
          className={cn(
            "fixed left-1/2 top-1/2 z-50 w-[95vw] max-w-xl -translate-x-1/2 -translate-y-1/2",
            "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg",
            "focus:outline-none space-y-4",
          )}
        >
          <div className="flex items-start justify-between gap-4">
            <div className="flex items-center gap-2">
              <ShieldQuestion aria-hidden className="size-5 text-[var(--primary)]" />
              <DialogPrimitive.Title className="text-base font-semibold">
                Request elevated view
              </DialogPrimitive.Title>
            </div>
            <DialogPrimitive.Close
              data-testid="elevated-view-dialog-close"
              className="rounded-[var(--radius-sm)] p-1 text-[var(--muted-foreground)] hover:bg-[var(--accent)]"
            >
              <X aria-hidden className="size-4" />
              <span className="sr-only">Close</span>
            </DialogPrimitive.Close>
          </div>
          <DialogPrimitive.Description className="text-sm text-[var(--muted-foreground)]">
            Audit-the-audit: this request is logged. v1 grants automatically; the
            operator-approval workflow lands in a follow-up.
          </DialogPrimitive.Description>

          <label className="block space-y-1">
            <span className="text-sm font-medium">Reason</span>
            <textarea
              data-testid="elevated-view-reason"
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              rows={3}
              placeholder="Compliance review request #CR-2024-08"
              className="w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
            />
            {reasonHint ? (
              <p className="text-xs text-[var(--muted-foreground)]" data-testid="elevated-view-reason-hint">
                {reasonHint}
              </p>
            ) : null}
          </label>

          <fieldset className="space-y-2" data-testid="elevated-view-categories">
            <legend className="text-sm font-medium">Categories to unlock</legend>
            <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
              {REQUESTABLE_CATEGORIES.map(({ permission, label }) => (
                <label key={permission} className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    data-testid={`elevated-view-cat-${permission}`}
                    checked={selected.has(permission)}
                    onChange={() => toggleCategory(permission)}
                    className="size-4 rounded border-[var(--border)]"
                  />
                  {label}
                </label>
              ))}
            </div>
          </fieldset>

          <label className="block space-y-1">
            <span className="text-sm font-medium">Duration</span>
            <select
              data-testid="elevated-view-duration"
              value={duration}
              onChange={(e) => setDuration(e.target.value as GrantDuration)}
              className="w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-2 text-sm"
            >
              {DURATION_OPTIONS.map((d) => (
                <option key={d} value={d}>
                  {GRANT_DURATION_LABELS[d]}
                </option>
              ))}
            </select>
          </label>

          {error ? (
            <p className="text-sm text-[var(--destructive)]" data-testid="elevated-view-error">
              {error}
            </p>
          ) : null}

          <div className="flex justify-end gap-2">
            <Button
              variant="ghost"
              onClick={() => onOpenChange(false)}
              data-testid="elevated-view-cancel"
            >
              Cancel
            </Button>
            <Button
              variant="primary"
              onClick={handleSubmit}
              disabled={submitDisabled}
              data-testid="elevated-view-submit"
            >
              {submitting ? <Loader2 className="animate-spin" aria-hidden /> : null}
              Submit request
            </Button>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
