"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { OnboardingShell } from "../_OnboardingShell";

export default function OnboardingWelcomePage() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);
  const selectedPlan = useOnboardingDraft((s) => s.selectedPlan);
  const setSelectedPlan = useOnboardingDraft((s) => s.setSelectedPlan);

  useEffect(() => {
    setStep(1);
  }, [setStep]);

  useEffect(() => {
    // Post-signup hop: if the visitor picked Pro on `/pricing`, route
    // straight to `/billing/subscribe?plan=pro` once they land here.
    // Clear the flag so a later return to `/onboarding/welcome` after
    // checkout doesn't re-fire the hop.
    if (selectedPlan === "pro") {
      setSelectedPlan(null);
      router.replace("/billing/subscribe?plan=pro");
    }
  }, [selectedPlan, setSelectedPlan, router]);

  return (
    <OnboardingShell
      step={1}
      stepLabel="WELCOME"
      eyebrow="STEP ONE · WELCOME"
      heroSegments={[
        { text: "Hire your " },
        { text: "first teammate.", accent: true },
      ]}
      sub={
        <>
          A guided interview so your teammate starts right the first time &mdash;
          with your role, playbook, and guardrails ready before they start.
        </>
      }
    >
      <section
        className="flex flex-col items-center gap-8 text-center"
        data-testid="onboarding-welcome"
      >
        <ul className="ob-pillars">
          <li className="ob-pillar" data-testid="onboarding-step-preview-setup">
            <p className="ob-pillar-label">Steps 2&ndash;3 &middot; SETUP</p>
            <p className="ob-pillar-body">
              Name your workspace, then connect your AI provider.
            </p>
          </li>
          <li
            className="ob-pillar"
            data-testid="onboarding-pillar-genesis-studio"
          >
            <p className="ob-pillar-label">Genesis &middot; Onboarding Studio</p>
            <p className="ob-pillar-body">
              Describe the role in plain English; Skycot synthesises charter,
              action vocabulary, memory schema, and eval suites &mdash; a real
              teammate ready to start work.
            </p>
          </li>
          <li className="ob-pillar" data-testid="onboarding-step-preview-hire">
            <p className="ob-pillar-label">Steps 4&ndash;5 &middot; HIRE</p>
            <p className="ob-pillar-body">
              Pick a vertical template, then describe your first teammate.
            </p>
          </li>
        </ul>
        <div className="ob-cta-row" data-testid="onboarding-genesis-aside">
          <button
            type="button"
            autoFocus
            className="ob-cta"
            data-testid="onboarding-welcome-continue"
            onClick={() => router.push("/onboarding/workspace")}
          >
            Get started
            <span className="ob-cta-kbd" aria-hidden>
              ↵
            </span>
          </button>
          <Link
            href="/genesis?return=/onboarding/welcome"
            data-testid="onboarding-genesis-link"
            className="ob-cta-ghost"
          >
            Open Genesis studio &rarr;
          </Link>
        </div>
      </section>
    </OnboardingShell>
  );
}
