import { forwardRef, type HTMLAttributes, type ReactNode } from "react";
import { ProgressDots } from "@/components/ui/ProgressDots";
import { cn } from "@/lib/utils";

export interface OnboardingStepProps extends HTMLAttributes<HTMLElement> {
  totalSteps: number;
  currentStep: number;
  stepLabels?: string[];
  headline: ReactNode;
  body?: ReactNode;
  actions?: ReactNode;
  onJumpBack?: (step: number) => void;
}

export const OnboardingStep = forwardRef<HTMLElement, OnboardingStepProps>(
  ({ className, totalSteps, currentStep, stepLabels, headline, body, actions, onJumpBack, children, ...props }, ref) => (
    <section
      ref={ref}
      data-testid="onboarding-step"
      className={cn("mx-auto w-full max-w-xl px-6 py-10 flex flex-col gap-6", className)}
      {...props}
    >
      <ProgressDots
        total={totalSteps}
        current={currentStep}
        stepLabels={stepLabels}
        onJumpBack={onJumpBack}
        label={`Onboarding progress — step ${currentStep} of ${totalSteps}`}
      />
      <header className="flex flex-col gap-2">
        <h2 className="text-2xl font-semibold tracking-tight text-[var(--foreground)]">
          {headline}
        </h2>
        {body ? (
          <p className="text-sm text-[var(--muted-foreground)] leading-relaxed">{body}</p>
        ) : null}
      </header>
      {children ? <div className="flex flex-col gap-4">{children}</div> : null}
      {actions ? <div className="flex flex-col sm:flex-row gap-2">{actions}</div> : null}
    </section>
  ),
);
OnboardingStep.displayName = "OnboardingStep";
