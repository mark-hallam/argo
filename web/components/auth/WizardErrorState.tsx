"use client";

import Link from "next/link";
import { AlertCircle, RotateCw } from "lucide-react";
import { buttonVariants } from "@/components/ui/Button";

/**
 * Error state for hire-wizard draft hydration failures.
 *
 * Replaces the previous white screen on a corrupted localStorage
 * draft or a `loadDraft()` rejection. Sits inside the hire route
 * so the wizard chrome around it is gone; the framing is a centred
 * editorial card with the same brand register as the rest of the
 * `/hire` flow (purple→pink gradient on the primary action).
 *
 * `onReset` clears the local draft and signals the page to retry;
 * the secondary action heads back to the team page. The brief calls
 * out: "Couldn't restore your draft — start over?" with a reset
 * action.
 */
export interface WizardErrorStateProps {
  variant?: "draft-hydration" | "worker-fetch";
  /** Clears the local draft and restarts the flow. */
  onReset?: () => void;
  /** Optional caller-supplied error for the live region. */
  errorMessage?: string;
}

const COPY: Record<
  NonNullable<WizardErrorStateProps["variant"]>,
  { title: string; description: string; reset: string }
> = {
  "draft-hydration": {
    title: "Couldn't restore your draft",
    description:
      "Your in-progress hire couldn't be loaded. Starting over takes about four minutes — your earlier answers aren't lost forever, they just couldn't be recovered.",
    reset: "Start over",
  },
  "worker-fetch": {
    title: "Couldn't load your new teammate",
    description:
      "Your teammate was hired, but we couldn't fetch their card. Head to your team to see them — they're already on the way to their first action.",
    reset: "Try again",
  },
};

export function WizardErrorState({
  variant = "draft-hydration",
  onReset,
  errorMessage,
}: WizardErrorStateProps) {
  const copy = COPY[variant];
  return (
    <div
      data-testid={`wizard-error-state-${variant}`}
      role="alert"
      style={{
        minHeight: "100vh",
        background: "#fafaff",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: "48px 24px",
        fontFamily: "Inter, sans-serif",
      }}
    >
      <div className="mx-auto max-w-md rounded-[var(--radius-lg)] border border-[var(--destructive)]/30 bg-[var(--destructive)]/5 px-6 py-12 text-center space-y-4">
        <div className="mx-auto grid size-12 place-items-center rounded-full bg-[var(--destructive)]/10 text-[var(--destructive)]">
          <AlertCircle aria-hidden />
        </div>
        <div className="space-y-1">
          <h2 className="text-base font-semibold text-[var(--foreground)]">
            {copy.title}
          </h2>
          <p className="text-sm text-[var(--muted-foreground)]">
            {copy.description}
          </p>
          {errorMessage ? (
            <p
              data-testid="wizard-error-state-detail"
              className="text-xs text-[var(--muted-foreground)]/80 pt-1"
            >
              {errorMessage}
            </p>
          ) : null}
        </div>
        <div className="flex flex-wrap items-center justify-center gap-3">
          {onReset ? (
            <button
              type="button"
              onClick={onReset}
              data-testid="wizard-error-state-reset"
              className="inline-flex items-center gap-2 rounded-[var(--radius-md)] px-4 py-2 text-sm font-semibold text-white"
              style={{
                background: "linear-gradient(135deg, #8b5cf6, #ec4899)",
                boxShadow: "0 4px 12px -2px rgba(139,92,246,0.4)",
              }}
            >
              <RotateCw className="size-3.5" />
              {copy.reset}
            </button>
          ) : null}
          <Link
            href="/team"
            className={buttonVariants({ variant: "secondary", size: "md" })}
          >
            Back to team
          </Link>
        </div>
      </div>
    </div>
  );
}
