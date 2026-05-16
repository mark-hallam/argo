"use client";

import { Sparkles } from "lucide-react";
import { usePathname } from "next/navigation";
import type { ReactNode } from "react";
import { ProgressDots } from "@/components/ui/ProgressDots";

const STEPS = [
  { slug: "welcome", label: "Welcome" },
  { slug: "workspace", label: "Workspace" },
  { slug: "connect-provider", label: "Connect provider" },
  { slug: "pick-vertical", label: "Pick a template" },
  { slug: "first-hire", label: "First hire" },
] as const;

function currentStep(pathname: string): number {
  const m = pathname.match(/\/onboarding\/([^/]+)/);
  const idx = STEPS.findIndex((s) => s.slug === m?.[1]);
  return idx >= 0 ? idx + 1 : 1;
}

export default function OnboardingLayout({ children }: { children: ReactNode }) {
  const pathname = usePathname() ?? "/onboarding/welcome";
  const step = currentStep(pathname);

  return (
    <div className="min-h-screen bg-[var(--background)] flex flex-col">
      <header className="sticky top-0 z-30 border-b border-[var(--border)] bg-[var(--background)]/85 backdrop-blur-md">
        <div className="mx-auto max-w-3xl px-4 sm:px-6 lg:px-8 py-4 flex items-center gap-4">
          <div
            aria-label="SKYCOT"
            data-testid="onboarding-brand"
            className="grid size-8 place-items-center rounded-[var(--radius-md)] bg-[var(--primary)] text-[var(--primary-foreground)] shadow-sm"
          >
            <Sparkles className="size-4" aria-hidden />
          </div>
          <div className="flex-1 flex justify-center">
            <ProgressDots
              total={STEPS.length}
              current={step}
              stepLabels={STEPS.map((s) => s.label)}
              label={`Onboarding — step ${step} of ${STEPS.length}`}
            />
          </div>
          <span
            className="text-xs font-medium tabular-nums text-[var(--muted-foreground)]"
            data-testid="onboarding-step-counter"
          >
            {step}/{STEPS.length}
          </span>
        </div>
      </header>
      <main className="flex-1 mx-auto w-full max-w-3xl px-4 sm:px-6 lg:px-8 py-10">
        {children}
      </main>
    </div>
  );
}
