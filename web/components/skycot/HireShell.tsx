"use client";

import { X } from "lucide-react";
import { useRouter } from "next/navigation";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/Button";
import { ProgressDots } from "@/components/ui/ProgressDots";

/**
 * Chrome-free shell for the hire onboarding flow: a header bar with
 * a brand square (returns to `/`), progress dots fed by the step
 * props, and an escape (X) button. No sidebar — the flow owns the
 * full viewport.
 *
 * `unsavedHire` (when true) triggers a `confirm()` dialog before
 * navigating away. Downstream E2 briefs wire this to the in-flight
 * draft state.
 */
export interface HireShellProps {
  children: ReactNode;
  /** Zero-indexed current step. */
  step: number;
  /** Total step count. */
  totalSteps: number;
  /** Show a confirm() before navigating away. */
  unsavedHire?: boolean;
}

const LEAVE_CONFIRM_MESSAGE =
  "Leave the hire flow? Your in-progress teammate setup will be discarded.";

export function HireShell({ children, step, totalSteps, unsavedHire }: HireShellProps) {
  const router = useRouter();

  function leaveFlow() {
    if (unsavedHire && typeof window !== "undefined") {
      const ok = window.confirm(LEAVE_CONFIRM_MESSAGE);
      if (!ok) return;
    }
    router.push("/");
  }

  return (
    <div className="min-h-screen bg-[var(--background)] flex flex-col">
      <header className="sticky top-0 z-30 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
        <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-4 flex items-center gap-4">
          <button
            type="button"
            onClick={leaveFlow}
            aria-label="Return to approvals"
            className="grid size-8 place-items-center rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] font-semibold text-sm shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
          >
            S
          </button>
          <div className="flex-1 flex justify-center">
            <ProgressDots
              total={totalSteps}
              current={step}
              label={`Hire flow — step ${step + 1} of ${totalSteps}`}
            />
          </div>
          <Button
            variant="ghost"
            size="icon"
            onClick={leaveFlow}
            aria-label="Exit hire flow"
            data-testid="hire-shell-escape"
          >
            <X />
          </Button>
        </div>
      </header>
      <main className="flex-1 mx-auto w-full max-w-4xl px-4 sm:px-6 lg:px-8 py-10">{children}</main>
    </div>
  );
}
