"use client";

import type { ReactNode } from "react";
import {
  WizardEditorialShell,
  type WizardStep,
} from "@/components/auth/WizardEditorialShell";
import { OnboardingFormStyles } from "@/components/skycot/onboarding/OnboardingFormStyles";

/**
 * Thin onboarding-side adapter around the locked F1 editorial-center
 * `WizardEditorialShell`. Each /onboarding step page wraps its form
 * content in this component so the 5-step flow shares the same Audiowide
 * hero + purple→pink gradient + warm radial-blob background + trust
 * footer + SAVE & EXIT affordance as /hire.
 *
 * Footer (back/continue) is hidden because every onboarding step keeps
 * its own form-validated submit button (provision call, key validate,
 * vertical select, hire commit) — driving those through the shell's
 * generic Continue would require rewriting every page's submit pipeline
 * and would lose the per-step skip/error semantics. The shell still
 * surfaces the SAVE & EXIT and Help affordances in the sticky top bar.
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
  return (
    <WizardEditorialShell
      step={step}
      completedSteps={completedSteps}
      stepLabel={stepLabel}
      eyebrow={eyebrow}
      heroSegments={heroSegments}
      sub={sub}
      hideFooter
    >
      <OnboardingFormStyles />
      {children}
    </WizardEditorialShell>
  );
}
