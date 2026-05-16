"use client";

import { ArrowRight } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { Button } from "@/components/ui/Button";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";

export default function OnboardingWelcomePage() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);

  useEffect(() => {
    setStep(1);
  }, [setStep]);

  return (
    <section
      className="flex flex-col items-center gap-6 text-center"
      data-testid="onboarding-welcome"
    >
      <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">
        Welcome to SKYCOT.
      </h1>
      <p className="max-w-xl text-base text-[var(--muted-foreground)] sm:text-lg">
        Let&apos;s hire your first worker. <span className="font-medium">(90 seconds.)</span>
      </p>
      <Button
        size="lg"
        data-testid="onboarding-welcome-continue"
        onClick={() => router.push("/onboarding/workspace")}
      >
        Continue
        <ArrowRight aria-hidden />
      </Button>
    </section>
  );
}
