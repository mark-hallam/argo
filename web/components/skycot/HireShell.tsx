"use client";

import { Sparkles, X } from "lucide-react";
import { useRouter } from "next/navigation";
import { useState, type ReactNode } from "react";
import { AlertDialog } from "@/components/ui/AlertDialog";
import { Button } from "@/components/ui/Button";
import { ProgressDots } from "@/components/ui/ProgressDots";

/**
 * Chrome-free shell for the hire onboarding flow.
 *
 * Renders:
 *  - Brand square (lucide `Sparkles`) top-left → clicks return to `/`.
 *  - `ProgressDots` showing 7 steps (per wireframe) with the active
 *    step highlighted and hover tooltips.
 *  - Escape × button top-right → same exit behavior as brand square.
 *  - Footer with Back + Continue buttons (Back disabled on Step 1).
 *
 * When `unsavedHire` is true, exit attempts (brand square, escape
 * button, and back-from-Step-1) raise an AlertDialog confirm; on
 * confirm the parent's `onDiscard` callback fires (clears the draft).
 *
 * Navigation is fully controlled: parents provide `onBack` /
 * `onContinue` per step. The shell does not assume a route shape.
 */
export interface HireShellProps {
  /** 1-based current step. 1–6 are valid. */
  step: 1 | 2 | 3 | 4 | 5 | 6;
  /** Step content (placeholder text in this brief; real content in E2-b..h). */
  children: ReactNode;
  /** Set true if the user has entered any in-progress hire data. */
  unsavedHire?: boolean;
  /** Disable the Continue button (no input yet, validation pending, etc.). */
  continueDisabled?: boolean;
  /** Override the default Back handler (`router.back()` to previous step). */
  onBack?: () => void;
  /** Required for Continue to be operable. */
  onContinue?: () => void;
  /** Called when the user confirms leaving an in-progress flow. */
  onDiscard?: () => void;
  /** Override the back button label (defaults to "Back"). */
  backLabel?: string;
  /** Override the continue button label (defaults to "Continue"). */
  continueLabel?: string;
  /**
   * Hide the Back/Continue footer entirely. Used by non-interactive
   * steps that auto-advance (e.g. Step 3 — Drafting transition).
   * The footer chrome (border, padding) is also dropped so the main
   * content can occupy the full vertical space.
   */
  hideFooter?: boolean;
}

const STEP_LABELS = [
  "Step 1 — Land",
  "Step 2 — Listen",
  "Step 3 — Drafting",
  "Step 4 — Introduce",
  "Step 5 — Connect",
  "Step 6 — First action",
  "Step 7 — Day-1 wrap",
] as const;
const TOTAL_STEPS = 7;

export function HireShell({
  step,
  children,
  unsavedHire = false,
  continueDisabled = false,
  onBack,
  onContinue,
  onDiscard,
  backLabel = "Back",
  continueLabel = "Continue",
  hideFooter = false,
}: HireShellProps) {
  const router = useRouter();
  const [confirmOpen, setConfirmOpen] = useState(false);

  function attemptLeave() {
    if (unsavedHire) {
      setConfirmOpen(true);
      return;
    }
    router.push("/");
  }

  function confirmDiscard() {
    onDiscard?.();
    router.push("/");
  }

  function handleBack() {
    if (onBack) {
      onBack();
      return;
    }
    router.back();
  }

  function handleJumpBack(targetStep: number) {
    if (targetStep === 1) {
      router.push("/hire");
    } else {
      router.push(`/hire/step/${targetStep}`);
    }
  }

  return (
    <div className="min-h-screen bg-[var(--background)] flex flex-col">
      <header className="sticky top-0 z-30 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
        <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-4 flex items-center gap-4">
          <button
            type="button"
            onClick={attemptLeave}
            aria-label="Return to approvals"
            data-testid="hire-shell-brand"
            className="grid size-8 place-items-center rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--ring)]"
          >
            <Sparkles className="size-4" aria-hidden />
          </button>
          <div className="flex-1 flex justify-center">
            <ProgressDots
              total={TOTAL_STEPS}
              current={step}
              stepLabels={[...STEP_LABELS]}
              onJumpBack={handleJumpBack}
              label={`Hire flow — step ${step} of ${TOTAL_STEPS}`}
            />
          </div>
          <Button
            variant="ghost"
            size="icon"
            onClick={attemptLeave}
            aria-label="Exit hire flow"
            data-testid="hire-shell-escape"
          >
            <X />
          </Button>
        </div>
      </header>
      <main className="flex-1 mx-auto w-full max-w-4xl px-4 sm:px-6 lg:px-8 py-10">
        {children}
      </main>
      {hideFooter ? null : (
        <footer className="sticky bottom-0 border-t border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
          <div className="mx-auto max-w-4xl px-4 sm:px-6 lg:px-8 py-4 flex items-center justify-between">
            <Button
              variant="ghost"
              disabled={step === 1}
              onClick={handleBack}
              data-testid="hire-shell-back"
            >
              {backLabel}
            </Button>
            <Button
              variant="primary"
              disabled={continueDisabled || !onContinue}
              onClick={() => onContinue?.()}
              data-testid="hire-shell-continue"
            >
              {continueLabel}
            </Button>
          </div>
        </footer>
      )}
      <AlertDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title="Discard your in-progress teammate?"
        description="Your draft will be cleared and you'll go back to the approvals inbox."
        confirmLabel="Discard and leave"
        cancelLabel="Keep working"
        onConfirm={confirmDiscard}
      />
    </div>
  );
}
