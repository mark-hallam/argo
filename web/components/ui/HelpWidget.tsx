"use client";

import * as DialogPrimitive from "@radix-ui/react-dialog";
import { HelpCircle, X } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { useCreateTicket } from "@/lib/support/use-create-ticket";

const SLA_PATH = "/sla";
const INPUT =
  "rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--background)] px-3 py-2 text-sm font-normal text-[var(--foreground)] focus:outline-none focus:ring-2 focus:ring-[var(--ring)]";
const LABEL = "flex flex-col gap-1 text-xs font-medium text-[var(--foreground)]";
const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function HelpWidget() {
  const [open, setOpen] = useState(false);
  const [email, setEmail] = useState("");
  const [message, setMessage] = useState("");
  const [done, setDone] = useState<string | null>(null);
  const mutation = useCreateTicket();
  const valid = EMAIL.test(email) && message.trim().length > 0;

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!valid || mutation.isPending) return;
    const res = await mutation.mutateAsync({ email, message }).catch(() => null);
    if (res) {
      setDone(res.ticketId ? `Ticket queued (#${res.ticketId}). We'll reply by email.` : "Ticket queued. We'll reply by email.");
      setEmail("");
      setMessage("");
    }
  }

  function handleOpenChange(next: boolean) {
    setOpen(next);
    if (!next) {
      setEmail("");
      setMessage("");
      setDone(null);
      mutation.reset();
    }
  }

  return (
    <DialogPrimitive.Root open={open} onOpenChange={handleOpenChange}>
      <DialogPrimitive.Trigger asChild>
        <button
          type="button"
          aria-label="Open help"
          data-testid="help-widget-trigger"
          className="fixed bottom-4 right-4 z-40 grid size-12 place-items-center rounded-full shadow-lg bg-[var(--primary)] text-[var(--primary-foreground)] transition-transform duration-[var(--duration-fast)] hover:scale-105 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
        >
          <HelpCircle className="size-5" aria-hidden />
        </button>
      </DialogPrimitive.Trigger>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm" />
        <DialogPrimitive.Content
          data-testid="help-widget-dialog"
          className="fixed left-1/2 top-1/2 z-50 w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2 rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)] p-6 shadow-lg focus:outline-none"
        >
          <div className="flex items-start justify-between gap-4">
            <div>
              <DialogPrimitive.Title className="text-base font-semibold text-[var(--foreground)]">
                Contact support
              </DialogPrimitive.Title>
              <DialogPrimitive.Description className="mt-1 text-sm text-[var(--muted-foreground)]">
                We reply by email.{" "}
                <a href={SLA_PATH} className="underline" target="_blank" rel="noreferrer">
                  Response SLAs
                </a>
                .
              </DialogPrimitive.Description>
            </div>
            <DialogPrimitive.Close aria-label="Close" className="rounded-[var(--radius-sm)] p-1 text-[var(--muted-foreground)] hover:text-[var(--foreground)]">
              <X className="size-4" />
            </DialogPrimitive.Close>
          </div>
          {done ? (
            <p data-testid="help-widget-confirmation" className="mt-4 text-sm text-[var(--foreground)]">{done}</p>
          ) : (
            <form className="mt-4 space-y-3" onSubmit={handleSubmit}>
              <label className={LABEL}>
                Email
                <input type="email" required autoComplete="email" value={email} onChange={(e) => setEmail(e.target.value)} data-testid="help-widget-email" className={INPUT} />
              </label>
              <label className={LABEL}>
                Message
                <textarea required rows={4} value={message} onChange={(e) => setMessage(e.target.value)} data-testid="help-widget-message" className={INPUT} />
              </label>
              {mutation.isError ? (
                <p role="alert" className="text-xs text-[var(--destructive)]">{mutation.error?.message ?? "Couldn't submit ticket."}</p>
              ) : null}
              <div className="flex justify-end pt-1">
                <Button type="submit" disabled={!valid} loading={mutation.isPending} data-testid="help-widget-submit">Send</Button>
              </div>
            </form>
          )}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
