"use client";

import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { useOnboardingDraft } from "@/lib/onboarding/use-onboarding-draft";
import { OnboardingShell } from "../_OnboardingShell";

export default function OnboardingWelcomePage() {
  const router = useRouter();
  const setStep = useOnboardingDraft((s) => s.setStep);

  useEffect(() => {
    setStep(1);
  }, [setStep]);

  return (
    <OnboardingShell
      step={1}
      stepLabel="WELCOME"
      eyebrow="WELCOME TO SKYCOT"
      heroSegments={[
        { text: "Build your " },
        { text: "AI workforce.", accent: true },
      ]}
      sub={
        <>
          In <strong>four minutes</strong> you&apos;ll onboard your first AI teammate &mdash;
          with your role, playbook, and guardrails ready before they start.
        </>
      }
    >
      <section
        className="flex flex-col items-center gap-8 text-center"
        data-testid="onboarding-welcome"
      >
        <ul className="ob-pillars">
          <li className="ob-pillar" data-testid="onboarding-pillar-audit-first">
            <p className="ob-pillar-label">Audit-first</p>
            <p className="ob-pillar-body">
              Every action signed and replayable &mdash; SOC 2-ready audit trail.
            </p>
          </li>
          <li className="ob-pillar" data-testid="onboarding-pillar-genesis-studio">
            <p className="ob-pillar-label">Onboard them like a human</p>
            <p className="ob-pillar-body">
              Describe the role in voice or text. Upload SOPs, wiki pages, and Slack
              threads; we verify they understand before they start.
            </p>
          </li>
          <li className="ob-pillar" data-testid="onboarding-pillar-enterprise-grade">
            <p className="ob-pillar-label">Enterprise-grade</p>
            <p className="ob-pillar-body">
              Multi-tenant, capability-based RBAC, GDPR Article 30 trails.
            </p>
          </li>
        </ul>
        <div className="ob-cta-row">
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
        </div>
      </section>
    </OnboardingShell>
  );
}
