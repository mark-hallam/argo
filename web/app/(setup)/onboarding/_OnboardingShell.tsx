"use client";

import type { ReactNode } from "react";
import {
  WizardEditorialShell,
  type WizardStep,
} from "@/components/auth/WizardEditorialShell";
import { OnboardingDemoPill } from "@/components/skycot/onboarding/OnboardingDemoPill";
import { OnboardingFormStyles } from "@/components/skycot/onboarding/OnboardingFormStyles";
import { useOnboardingSaveProgress } from "@/lib/onboarding/use-save-progress";

/**
 * Thin onboarding-side adapter around the locked F1 editorial-center
 * `WizardEditorialShell`. Each /onboarding step page wraps its form
 * content in this component so the 5-step flow shares the same Audiowide
 * hero + purple→pink gradient + warm radial-blob background + trust
 * footer + exit affordance as /hire.
 *
 * Footer (back/continue) is hidden because every onboarding step keeps
 * its own form-validated submit button (provision call, key validate,
 * vertical select, hire commit) — driving those through the shell's
 * generic Continue would require rewriting every page's submit pipeline
 * and would lose the per-step skip/error semantics. The shell still
 * surfaces the Exit setup and Help affordances in the sticky top bar.
 *
 * The top-bar exit button is wired through the new shell exit-confirm
 * props (UX-audit F10, P2 — fix-onboarding-save-and-exit-button-safety):
 * the label is "Exit setup" (not "SAVE & EXIT"), exits always raise a
 * confirmation dialog before navigating away (`forceExitConfirm`), the
 * dialog explains progress is already saved, and the destination is
 * `/workers` if the workspace was provisioned in step 2 or `/landing`
 * otherwise. The /hire flow keeps its prior copy via defaults.
 */

interface HeroSpan {
  text: string;
  accent?: boolean;
}

export interface OnboardingShellProps {
  step: WizardStep;
  /**
   * Override prior-step completion count. Defaults to `step - 1` so the
   * active pip renders distinct from completed pips (UX-audit F9, P1).
   */
  completedSteps?: number;
  stepLabel: string;
  eyebrow: string;
  heroSegments: HeroSpan[];
  sub?: ReactNode;
  children: ReactNode;
}

export function OnboardingShell({
  step,
  completedSteps,
  stepLabel,
  eyebrow,
  heroSegments,
  sub,
  children,
}: OnboardingShellProps) {
  const { destination, isProvisioned } = useOnboardingSaveProgress();
  const exitDialogDescription = isProvisioned
    ? "Your progress is saved. You can resume the wizard any time from the dashboard."
    : "Your progress is saved on this browser. Open /onboarding/welcome to resume.";
  return (
    <WizardEditorialShell
      step={step}
      completedSteps={completedSteps}
      stepLabel={stepLabel}
      eyebrow={eyebrow}
      heroSegments={heroSegments}
      sub={sub}
      hideFooter
      exitLabel="Exit setup"
      exitDestination={destination}
      forceExitConfirm
      exitDialogTitle="Exit setup and save your progress?"
      exitDialogDescription={exitDialogDescription}
      exitConfirmLabel="Save & exit"
      exitCancelLabel="Stay in setup"
      exitConfirmVariant="primary"
    >
      <OnboardingFormStyles />
      {children}
      <OnboardingDemoPill />
    </WizardEditorialShell>
  );
}
