"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { Ban, Snowflake } from "lucide-react";
import { useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/Button";
import { cn } from "@/lib/utils";
import { freezeWorker, unfreezeWorker } from "@/lib/api/workers";

/**
 * Operator-facing freeze / unfreeze control for the worker profile.
 *
 * Two states:
 * - **Not frozen**: red "Freeze worker" button. Opens a confirm modal
 *   that requires typing the worker id (mirrors GitHub's "delete
 *   repo" flow). The modal also takes a free-text reason that is
 *   forwarded to the gateway.
 * - **Frozen**: secondary "Unfreeze worker" button. Single-click — the
 *   safe direction does not need name-typing confirm.
 *
 * Capability gating is the page's responsibility: the parent only
 * renders this component when the session has `worker.freeze`. The
 * gateway also enforces the capability server-side, so a leaked
 * button is not a security boundary.
 *
 * The modal is composed inline from `@radix-ui/react-dialog` rather
 * than reusing `AlertDialog` because the confirm button must remain
 * disabled until the typed id matches (AlertDialog has no
 * `confirmDisabled` knob).
 */
export interface FreezeButtonProps {
  workerId: string;
  /** Currently-frozen flag — flips the label + bypasses name-typing. */
  isFrozen: boolean;
}

export function FreezeButton({ workerId, isFrozen }: FreezeButtonProps) {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [typedId, setTypedId] = useState("");
  const [reason, setReason] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();
  const [submitting, setSubmitting] = useState(false);

  const idConfirmed = typedId.trim() === workerId;
  const reasonReady = reason.trim().length > 0;
  const canSubmit = idConfirmed && reasonReady && !submitting;

  function reset() {
    setTypedId("");
    setReason("");
    setError(null);
  }

  async function runFreeze() {
    setError(null);
    setSubmitting(true);
    try {
      await freezeWorker(workerId, reason.trim());
      setOpen(false);
      reset();
      startTransition(() => router.refresh());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Couldn't freeze worker");
    } finally {
      setSubmitting(false);
    }
  }

  async function runUnfreeze() {
    setError(null);
    setSubmitting(true);
    try {
      await unfreezeWorker(workerId);
      startTransition(() => router.refresh());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Couldn't unfreeze worker");
    } finally {
      setSubmitting(false);
    }
  }

  if (isFrozen) {
    return (
      <div className="flex flex-col items-end gap-1">
        <Button
          variant="secondary"
          size="sm"
          data-testid="worker-unfreeze-button"
          onClick={runUnfreeze}
          loading={submitting || pending}
        >
          <Snowflake aria-hidden />
          Unfreeze worker
        </Button>
        {error ? (
          <p role="alert" className="text-xs text-[var(--destructive)]">
            {error}
          </p>
        ) : null}
      </div>
    );
  }

  return (
    <div className="flex flex-col items-end gap-1">
      <Button
        variant="destructive"
        size="sm"
        data-testid="worker-freeze-button"
        onClick={() => setOpen(true)}
      >
        <Ban aria-hidden />
        Freeze worker
      </Button>
      <DialogPrimitive.Root
        open={open}
        onOpenChange={(next) => {
          setOpen(next);
          if (!next) reset();
        }}
      >
        <DialogPrimitive.Portal>
          <DialogPrimitive.Overlay
            className="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm"
          />
          <DialogPrimitive.Content
            role="alertdialog"
            aria-modal="true"
            data-testid="worker-freeze-dialog"
            className={cn(
              "fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2",
              "rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-5 sm:p-6 shadow-lg",
              "focus:outline-none",
            )}
          >
            <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
              Freeze this worker?
            </DialogPrimitive.Title>
            <DialogPrimitive.Description className="mt-2 text-sm text-[var(--muted-foreground)]">
              The worker will refuse new sessions until you unfreeze it. In-flight
              turns finish; no state is destroyed.
            </DialogPrimitive.Description>
            <div className="mt-4 space-y-3">
              <label className="block text-xs font-medium text-[var(--foreground)]">
                Type the worker id{" "}
                <code className="rounded bg-[var(--muted)] px-1 py-0.5 text-[var(--foreground)]">
                  {workerId}
                </code>{" "}
                to confirm:
                <input
                  type="text"
                  value={typedId}
                  onChange={(e) => setTypedId(e.target.value)}
                  data-testid="worker-freeze-confirm-id"
                  aria-label="Worker id confirmation"
                  className="mt-1 block w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-[var(--ring)]"
                  autoComplete="off"
                />
              </label>
              <label className="block text-xs font-medium text-[var(--foreground)]">
                Reason (required, recorded in audit):
                <textarea
                  value={reason}
                  onChange={(e) => setReason(e.target.value)}
                  data-testid="worker-freeze-reason"
                  aria-label="Freeze reason"
                  rows={2}
                  className="mt-1 block w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-[var(--ring)]"
                />
              </label>
              {error ? (
                <p role="alert" className="text-xs text-[var(--destructive)]">
                  {error}
                </p>
              ) : null}
            </div>
            <div className="mt-6 flex justify-end gap-2">
              <Button
                variant="ghost"
                size="sm"
                data-testid="worker-freeze-cancel"
                onClick={() => setOpen(false)}
              >
                Cancel
              </Button>
              <Button
                variant="destructive"
                size="sm"
                data-testid="worker-freeze-confirm"
                disabled={!canSubmit}
                loading={submitting}
                onClick={runFreeze}
              >
                Freeze worker
              </Button>
            </div>
          </DialogPrimitive.Content>
        </DialogPrimitive.Portal>
      </DialogPrimitive.Root>
    </div>
  );
}
